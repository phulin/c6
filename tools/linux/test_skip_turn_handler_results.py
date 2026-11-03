#!/usr/bin/env python3
"""Protocol rejection tests only; these synthetic rows are never candidate inputs."""
import copy
import unittest

import skip_turn_handler_results as checker

PROFILE = 'a' * 64


def fixture():
    units = [dict(owner=0, ordinal=0, id=65536), dict(owner=0, ordinal=1, id=65537)]
    request = dict(players=[dict(owner=0, units=dict(entries=units), formations=dict(entries=[]))])
    calculated = dict(count_results=[], unit_binding_results=[], handler_results=[])
    for index, actor in enumerate(units):
        for direction in range(-1, 6):
            row = dict(actor, kind='count_result', member_kind='unit', direction=direction,
                       plot_present=index == 0, anchor=[0, 0] if index == 0 else [-1, -1],
                       anchor_valid=index == 0, layer=0 if index == 0 else -1)
            if index == 0:
                row.update(plot=[0, 0], count=0, bool_argument=False)
            calculated['count_results'].append(row)
        calculated['unit_binding_results'].append(dict(actor, kind='unit_binding_result', formation_id=100 + index, definition_index=0))
        calculated['handler_results'].append(dict(actor, kind='handler_result', result=index == 0, native_compared=False))
    count_rows = [dict(kind='header', schema='c6-linux-placement-handler-count-expected-v1', profile_sha256=PROFILE,
                       answers_are_inputs=False, native_handler_query_available=True)]
    for index, binding in enumerate(calculated['unit_binding_results']):
        count_rows.append(dict(binding, kind='unit_binding_expected'))
        count_rows.extend(dict(r, kind='count_expected') for r in calculated['count_results'][index * 7:(index + 1) * 7])
    count_rows.append(dict(kind='summary', status=0))
    handlers = [dict(kind='header', schema='c6-linux-skip-turn-handler-expected-v1', profile_sha256=PROFILE,
                     answers_are_inputs=False, full_manager_legality=False)]
    for repeat in range(2):
        for visible in (False, True):
            handlers.append(dict(kind='handler_no_call_control', unit_present=False, plot_present=False,
                                 visible_only=visible, repeat=repeat, operation_id=9,
                                 operation_hash=checker.OPERATION_HASH, allowed=False, original_leaf_called=False))
        for index, actor in enumerate(units):
            for variant in (('current', 'null') if index == 0 else ('null',)):
                for visible in (False, True):
                    handlers.append(dict(actor, kind='handler_expected', unit_present=True, plot_variant=variant,
                                         plot_present=variant == 'current', visible_only=visible, repeat=repeat,
                                         operation_id=9, operation_hash=checker.OPERATION_HASH,
                                         allowed=index == 0, original_leaf_called=True))
    handlers.append(dict(kind='summary', status=0, units=2, original_leaf_calls=12, no_call_controls=4, repeats=2))
    return request, calculated, count_rows, handlers


class RejectionTests(unittest.TestCase):
    def setUp(self):
        self.request, self.calculated, self.count_rows, self.handler_rows = fixture()
        self.units, self.current = checker.candidate_roster(self.request, self.calculated)

    def compare(self, rows):
        return checker.compare_handlers(self.calculated, self.units, self.current, rows, PROFILE, 9)

    def test_complete_matrix_and_counts(self):
        self.assertEqual(checker.compare_counts(self.calculated, self.count_rows, PROFILE), [])
        mismatches, matrix = self.compare(self.handler_rows)
        self.assertEqual(mismatches, [])
        self.assertEqual((matrix['original_leaf_calls'], matrix['no_call_controls'], matrix['placed_units'], matrix['unplaced_units']), (12, 4, 1, 1))
        self.assertEqual((matrix['native_true'], matrix['native_false']), (8, 4))

    def test_handler_identity_matrix_mutations_reject(self):
        mutations = dict(owner=1, ordinal=1, id=65539, unit_present=False, plot_variant='adjacent',
                         plot_present=False, visible_only=True, repeat=1, operation_id=10,
                         operation_hash=0, original_leaf_called=False, kind='unsupported')
        for key, value in mutations.items():
            with self.subTest(key=key):
                rows = copy.deepcopy(self.handler_rows)
                rows[3][key] = value
                with self.assertRaises(ValueError):
                    self.compare(rows)

    def test_all_wrong_operation_ids_reject(self):
        rows = copy.deepcopy(self.handler_rows)
        for row in rows[1:-1]:
            row['operation_id'] = 10
        with self.assertRaises(ValueError):
            self.compare(rows)

    def test_missing_duplicate_reordered_rows_reject(self):
        cases = []
        missing = copy.deepcopy(self.handler_rows)
        missing.pop(3)
        cases.append(missing)
        duplicate = copy.deepcopy(self.handler_rows)
        duplicate.insert(3, duplicate[3])
        cases.append(duplicate)
        swapped = copy.deepcopy(self.handler_rows)
        swapped[3], swapped[4] = swapped[4], swapped[3]
        cases.append(swapped)
        for rows in cases:
            with self.assertRaises(ValueError):
                self.compare(rows)

    def test_no_call_controls_never_count_native_false(self):
        rows = copy.deepcopy(self.handler_rows)
        rows[1]['allowed'] = True
        with self.assertRaises(ValueError):
            self.compare(rows)

    def test_allowed_mismatch_reported(self):
        rows = copy.deepcopy(self.handler_rows)
        rows[3]['allowed'] = False
        mismatches, _ = self.compare(rows)
        self.assertEqual(len(mismatches), 1)

    def test_bool_integer_confusion_rejects(self):
        for key in ('allowed', 'unit_present', 'visible_only', 'original_leaf_called'):
            rows = copy.deepcopy(self.handler_rows)
            rows[3][key] = int(rows[3][key])
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.compare(rows)

    def test_unsupported_is_not_false(self):
        calculated = copy.deepcopy(self.calculated)
        calculated['handler_results'][0].pop('result')
        calculated['handler_results'][0]['unsupported'] = 'missing Formation'
        with self.assertRaises(ValueError):
            checker.candidate_roster(self.request, calculated)

    def test_missing_formation_rejects(self):
        calculated = copy.deepcopy(self.calculated)
        calculated['unit_binding_results'][0]['formation_id'] = None
        with self.assertRaisesRegex(ValueError, '244'):
            checker.candidate_roster(self.request, calculated)

    def test_count_direction_order_ids_plot_membership_reject(self):
        for key, value in dict(direction=1, id=0, member_kind='formation', plot_present=False, bool_argument=True).items():
            rows = copy.deepcopy(self.count_rows)
            rows[2][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                checker.compare_counts(self.calculated, rows, PROFILE)

    def test_binding_value_mismatch_reported(self):
        rows = copy.deepcopy(self.count_rows)
        rows[1]['formation_id'] += 1
        self.assertEqual(len(checker.compare_counts(self.calculated, rows, PROFILE)), 1)

    def test_duplicate_json_keys_reject(self):
        with self.assertRaises(ValueError):
            checker.base.parse('{"kind":"handler_expected","kind":"summary"}')


if __name__ == '__main__':
    unittest.main()
