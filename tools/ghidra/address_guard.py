"""Address identity checks around upstream MCP's permissive listing lookup."""
import re

def numeric_address(value):
    return int(value,16) if isinstance(value,str) and re.fullmatch(r'(?:0[xX])?[0-9a-fA-F]+',value) else None

def require_instruction_start(requested, listing):
    expected=numeric_address(requested)
    first=listing.splitlines()[0].split()[0] if listing.strip() else None
    actual=numeric_address(first)
    if actual!=expected:
        raise ValueError(f'No defined instruction at requested address {requested}; upstream listing starts at {first}. Create/verify the instruction boundary or inspect original binary bytes.')

def require_function_entry(requested, actual):
    expected=numeric_address(requested)
    if expected is not None and numeric_address(str(actual))!=expected:
        raise ValueError(f'Requested address {requested} is not a function entry; upstream resolved containing function at {actual}. Request that entry explicitly or establish the requested boundary.')

def install():
    from pyghidra_mcp.tools import GhidraTools
    original_disassemble=GhidraTools.disassemble
    original_decompile=GhidraTools.decompile_function_by_name_or_addr
    def disassemble(self,address,count=20,include_bytes=False):
        result=original_disassemble(self,address,count=count,include_bytes=include_bytes)
        require_instruction_start(address,result.listing)
        return result
    def decompile(self,name_or_address,timeout=30):
        function=self.find_function(name_or_address)
        require_function_entry(name_or_address,function.getEntryPoint())
        return original_decompile(self,name_or_address,timeout=timeout)
    GhidraTools.disassemble=disassemble
    GhidraTools.decompile_function_by_name_or_addr=decompile
