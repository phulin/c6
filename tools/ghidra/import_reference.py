#!/usr/bin/env python3
"""Apply pinned XP2 reference names through PyGhidra, with binary identity gates.

Stop the MCP project owner before invoking this standalone script. It never runs
analysis or imports Linux structure layouts as Windows types.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'evidence/reference/upstream'
PIN = '5af389445118bed2bdebf7a8a42b219c8fba5175'
TARGET_SHA256 = '324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c'
TARGET_SIZE = 12940384
STRONG = {'unique', 'unique-reordered', 'resolved', 'datashift-verified'}
DERIVED = {'callgraph-callee', 'callgraph-caller', 'vtable-derived', 'datashift-probable'}


def verify_source(source):
    manifest = json.loads((ROOT / 'tools/reference/manifest.json').read_text())
    if manifest['commit'] != PIN:
        raise ValueError('Unexpected reference revision')
    native_path = 'reference/data/native.json'
    if hashlib.sha256((source / native_path).read_bytes()).hexdigest() != manifest['sha256'][native_path]:
        raise ValueError('Reference native signature checksum mismatch')
    tsv = source / 'data/old_to_new_offsets.tsv'
    if hashlib.sha256(tsv.read_bytes()).hexdigest() != manifest['sha256']['data/old_to_new_offsets.tsv']:
        raise ValueError('Reference address map checksum mismatch')
    identity_path = 'tools/frida-live/builds/15296837/README.md'
    identity_bytes = (source / identity_path).read_bytes()
    if hashlib.sha256(identity_bytes).hexdigest() != manifest['sha256'][identity_path]:
        raise ValueError('Upstream identity artifact checksum mismatch')
    identity = identity_bytes.decode()
    if TARGET_SHA256.upper() not in identity.upper():
        raise ValueError('Upstream binary identity statement missing')
    return tsv


def annotate(program, source=SOURCE, binary=None):
    """Called while the caller owns the project; owns its transaction, not saving."""
    from ghidra.program.model.symbol import SourceType
    from ghidra.program.model.listing import CodeUnit
    tsv = verify_source(Path(source))
    sha = str(program.getExecutableSHA256()).lower()
    if sha != TARGET_SHA256:
        raise ValueError('Program SHA256 mismatch: ' + sha)
    if binary is not None:
        data = Path(binary).read_bytes()
        if len(data) != TARGET_SIZE or hashlib.sha256(data).hexdigest() != TARGET_SHA256:
            raise ValueError('Input file identity mismatch')
    symbols = program.getSymbolTable()
    listing = program.getListing()
    functions = program.getFunctionManager()
    counts = Counter()
    categories = Counter()
    details = []
    native_signatures = {}
    for row in json.loads((Path(source) / 'reference/data/native.json').read_text()):
        rva = row.get('builds', {}).get('current', {}).get('rva')
        if rva and row.get('signatures'):
            native_signatures.setdefault(rva.lower(), []).extend(row['signatures'])
    transaction = program.startTransaction('Pinned cru121 reference names ' + PIN[:12])
    ok = False
    try:
        namespaces = {}
        for category in sorted(STRONG | DERIVED):
            name = 'Ref_civ6_' + category.replace('-', '_')
            namespace = symbols.getNamespace(name, program.getGlobalNamespace())
            if namespace is None:
                namespace = symbols.createNameSpace(program.getGlobalNamespace(), name, SourceType.IMPORTED)
            namespaces[category] = namespace
        for line in tsv.read_text().splitlines():
            if not line or line.startswith('#'):
                continue
            fields = line.split('\t')
            if len(fields) != 5:
                counts['rejected_bad_row'] += 1
                continue
            old, current, category, size, original = fields
            if old == 'old_rva':
                continue
            counts['rows'] += 1
            if category not in STRONG | DERIVED:
                counts['rejected_category'] += 1
                continue
            if not re.fullmatch(r'0x[0-9a-fA-F]+', current):
                counts['rejected_non_single_address'] += 1
                continue
            address = program.getImageBase().add(int(current, 16))
            block = program.getMemory().getBlock(address)
            if block is None or not block.isExecute():
                counts['rejected_non_executable'] += 1
                continue
            # Preserve full original names in comments; Ghidra labels need safe spelling.
            name = re.sub(r'[^A-Za-z0-9_$]', '_', original)
            name = name[:700] + '__rva_' + current[2:]
            f = functions.getFunctionAt(address)
            previous_name = str(f.getName()) if f is not None else None
            previous_source = f.getSymbol().getSource() if f is not None else None
            existing = symbols.getSymbol(name, address, namespaces[category])
            if existing is None:
                symbols.createLabel(address, name, namespaces[category], SourceType.IMPORTED)
                counts['labels_added'] += 1
            else:
                counts['labels_existing'] += 1
            categories[category] += 1
            # Only rename auto-generated defaults. Existing imported/user names survive.
            if f is not None and previous_source == SourceType.DEFAULT and category in STRONG:
                f.setName(name, SourceType.IMPORTED)
                counts['default_functions_named'] += 1
            elif f is not None:
                if previous_source != SourceType.DEFAULT and str(f.getName()) != previous_name:
                    f.setName(previous_name, previous_source)
                counts['existing_function_names_preserved'] += 1
            else:
                counts['label_without_function'] += 1
            note = 'cru121-reference ' + PIN + '\n' + original + '\nmap=' + category + '; symbol_rva=' + old + '; target_rva=' + current + '; symbol_size=' + size + '\nRVA identity verified by module SHA256; mapping confidence is upstream static evidence, not runtime verification.'
            if current.lower() in native_signatures:
                note += '\nRecovered reference signatures (descriptive only; native ABI unverified; not applied): ' + json.dumps(native_signatures[current.lower()], sort_keys=True)
                counts['signature_comments'] += 1
            previous = listing.getComment(CodeUnit.PRE_COMMENT, address)
            if previous is None or note not in str(previous):
                listing.setComment(address, CodeUnit.PRE_COMMENT, (str(previous) + '\n\n' if previous else '') + note)
                counts['comments_added'] += 1
            details.append({'rva':current, 'map':category, 'original':original})
        info = program.getOptions('Reference provenance')
        for key, value in {
            'repository':'https://github.com/cru121/civ6-gamecore-reference',
            'revision':PIN,
            'license':'CC-BY-4.0; attribution cru121, Civ6-GameCore-reference',
            'target_sha256':TARGET_SHA256,
            'identity_evidence':'tools/frida-live/builds/15296837/README.md',
            'layout_source':'reference/data/layouts.json: Linux DWARF only, not imported as Windows structures',
            'signature_policy':'reference/data/native.json recovered signatures appended as descriptive comments; never applied as native prototypes',
            'mapping_policy':'strong and derived categories labelled separately; only DEFAULT functions renamed from strong categories; no Linux layouts imported',
            'counts':json.dumps(dict(counts), sort_keys=True),
        }.items():
            info.setString(key, value)
        ok = True
    finally:
        program.endTransaction(transaction, ok)
    return {'reference_revision':PIN, 'target_sha256':sha, 'counts':dict(counts), 'categories':dict(categories), 'applied':details}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--project-dir', type=Path, required=True)
    parser.add_argument('--project-name', required=True)
    parser.add_argument('--program', required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    import pyghidra
    pyghidra.start()
    from ghidra.base.project import GhidraProject
    project = GhidraProject.openProject(str(args.project_dir.resolve()), args.project_name, False)
    try:
        program = project.openProgram('/', args.program, False)
        result = annotate(program, binary=args.binary)
        project.save(program)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({k:v for k,v in result.items() if k != 'applied'}, sort_keys=True))
    finally:
        project.close()


if __name__ == '__main__':
    main()
