#!/usr/bin/env python3
"""Offline coverage-accounting regressions; fixtures do not execute native code."""
import unittest

from movement_expanded_corpus import LIMIT, coverage


class CoverageAccounting(unittest.TestCase):
    def setUp(self):
        self.header = {'formation_limit': LIMIT, 'execute': False}
        self.summary = {'formations': 1, 'units_seen': 1, 'skipped': 0,
                        'truncated': False}
        self.case = {'owner': 0, 'formation_id': 1, 'unit_id': 1,
                     'formation_domain': 0, 'formation_layer': -1,
                     'direction': 0, 'from': {'x': 0, 'y': 1},
                     'members': [{'id': 1, 'found': False}]}
        self.map = {'map_width': 3, 'map_height': 3,
                    'wrap_x': True, 'wrap_y': False}

    def test_missing_member_retained_without_escort_claim(self):
        result = coverage(self.header, self.summary, [self.case], self.map)
        self.assertEqual(result['missing_member_cases'], 1)
        self.assertEqual(result['escort_member_cases'], 0)
        self.assertEqual(result['cases_by_layer'], {'-1': 1})
        self.assertEqual(result['absent_directions_for_observed_formations'], 5)

    def test_wrapped_coordinate_edge_is_not_open_border(self):
        result = coverage(self.header, self.summary, [self.case], self.map)
        self.assertEqual(result['source_coordinate_edge_cases'], 1)
        self.assertEqual(result['source_open_border_cases'], 0)
        self.map['wrap_x'] = False
        result = coverage(self.header, self.summary, [self.case], self.map)
        self.assertEqual(result['source_open_border_cases'], 1)

    def test_over_budget_summary_rejected(self):
        self.summary['formations'] = LIMIT + 1
        with self.assertRaisesRegex(ValueError, 'formation accounting/budget differs'):
            coverage(self.header, self.summary, [self.case], self.map)


if __name__ == '__main__':
    unittest.main()
