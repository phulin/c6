#!/usr/bin/env python3
"""Launch upstream MCP with optional whole-program semantic indexing.

Large DLLs have tens of thousands of functions. Eager semantic indexing can
hold shutdown for hours. String indexes and direct decompilation stay enabled.
Set C6_GHIDRA_CODE_INDEX=1 to opt into upstream whole-program code indexing.
"""
import os
from pyghidra_mcp.indexing_mixin import IndexingMixin

if os.environ.get('C6_GHIDRA_CODE_INDEX') != '1':
    original_index = IndexingMixin._index_program
    def index_strings(self, program_info, *, code=True, strings=True):
        return original_index(self, program_info, code=False, strings=strings)
    IndexingMixin._index_program = index_strings

from pyghidra_mcp.server import main
main()
