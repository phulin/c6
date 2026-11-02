#!/usr/bin/env python3
"""Read-only ELF inventory; never loads or executes supplied game binaries."""
import argparse,hashlib,json,pathlib,re,subprocess

def readelf(path,*args):
    r=subprocess.run(['readelf','--wide',*args,str(path)],capture_output=True,text=True)
    return r.stdout if r.returncode==0 else ''

def sha256(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):h.update(block)
    return h.hexdigest()

def inventory(path,root):
    header=readelf(path,'--file-header');dynamic=readelf(path,'--dynamic');sections=readelf(path,'--section-headers');notes=readelf(path,'--notes');symbols=readelf(path,'--dyn-syms')
    fields={}
    for key in ['Class','Data','Type','Machine']:
        m=re.search(r'^\s*'+key+r':\s*(.*)$',header,re.M)
        if m:fields[key.lower()]=m[1]
    deps=re.findall(r'\(NEEDED\).*?\[(.*?)\]',dynamic)
    undefined=[];exports=[]
    for line in symbols.splitlines():
        parts=line.split()
        if len(parts)>=8 and parts[0].rstrip(':').isdigit():
            name=parts[7]
            if parts[6]=='UND':undefined.append(name)
            elif parts[4] in ('GLOBAL','WEAK'):exports.append(name)
    wanted=['DllCreateGameContext','DllDestroyGameContext']
    return {'path':str(path.relative_to(root)),'bytes':path.stat().st_size,'sha256':sha256(path),**fields,'build_ids':re.findall(r'Build ID:\s*(\S+)',notes),'needed':deps,'soname':re.findall(r'\(SONAME\).*?\[(.*?)\]',dynamic),'runtime_paths':re.findall(r'\((?:RPATH|RUNPATH)\).*?\[(.*?)\]',dynamic),'has_debug_info':bool(re.search(r'\.(?:zdebug_info|debug_info)\b',sections)),'has_symtab':bool(re.search(r'\.symtab\b',sections)),'has_debuglink':bool(re.search(r'\.gnu_debuglink\b',sections)),'undefined_dynamic_symbols':undefined,'exported_dynamic_symbol_count':len(exports),'context_exports':[x for x in exports if any(w in x for w in wanted)],'host_provider_candidates':sum(any(t in x for t in ['Platform','TypedVariant','Localization','GameCore','String']) for x in exports)}

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('root',type=pathlib.Path);ap.add_argument('--output',type=pathlib.Path);args=ap.parse_args();root=args.root.resolve();artifacts=[]
    for path in sorted(root.rglob('*')):
        if not path.is_file() or path.is_symlink():continue
        try:
            with path.open('rb') as f:magic=f.read(4)
        except OSError:continue
        if magic==b'\x7fELF':artifacts.append(inventory(path,root))
    result={'schema':1,'root':str(root),'method':'readelf only; no ldd/dlopen or artifact execution','artifacts':artifacts,'warning':'DT_NEEDED is not a complete host contract. GameCore may depend on symbols exported by Civ6. Linux version/layout are independent of Windows XP2.'}
    data=json.dumps(result,indent=2)+'\n'
    if args.output:args.output.write_text(data)
    else:print(data,end='')

if __name__=='__main__':main()
