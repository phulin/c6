#!/usr/bin/env python3
"""Regression cases for silently skipped instruction/function addresses."""
import unittest
from address_guard import require_instruction_start,require_function_entry

class AddressIdentity(unittest.TestCase):
    def test_real_listing_skips_are_rejected(self):
        for requested,returned in [('0x180672e10','1806d8de0 RET'),('0x180013230','180033b30 PUSH RBP')]:
            with self.assertRaisesRegex(ValueError,'No defined instruction'):
                require_instruction_start(requested,returned)
    def test_exact_entries_accept_normalized_hex(self):
        require_instruction_start('0x1805fa040','1805fa040 PUSH RBX\n1805fa042 RET')
        require_function_entry('0X1805FA040','1805fa040')
    def test_empty_listing_and_containing_function_rejected(self):
        with self.assertRaises(ValueError):require_instruction_start('0x1805fa040','')
        with self.assertRaisesRegex(ValueError,'containing function'):require_function_entry('0x1805fa041','1805fa040')
    def test_symbol_names_retain_upstream_resolution(self):
        require_function_entry('Random::Get','1805fa040')

if __name__=='__main__':unittest.main()
