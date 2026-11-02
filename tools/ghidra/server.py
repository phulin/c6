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


# Targeted mode serves persisted symbols/instructions without scheduling a full
# reanalysis. Keep the actual Ghidra analyzed flag unchanged in metadata.
if os.environ.get('C6_GHIDRA_AUTO_ANALYZE') != '1':
    from pyghidra_mcp.context import PyGhidraContext
    def targeted_analysis(self, *args, **kwargs):
        return None
    def existing_program(self, binary_name):
        info = self._lookup_program_info(binary_name)
        if info is None:
            raise ValueError(f"Unknown binary {binary_name}; available: {list(self.programs)}")
        return info
    PyGhidraContext.analyze_project = targeted_analysis
    PyGhidraContext.get_program_info = existing_program

from pyghidra_mcp.server import main
main()
