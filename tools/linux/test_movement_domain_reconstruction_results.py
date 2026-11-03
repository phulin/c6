"""Source-only checker acceptance/error fixtures; no original code execution."""
import copy
import unittest

from movement_domain_reconstruction_results import compare, parse


def fixture():
    calculated = dict(plots=[
        dict(x=0, y=0, has_attributes=True, coastal_min_one=True),
        dict(x=1, y=0, has_attributes=False, coastal_min_one=False)],
        domain_reconstruction=dict(masks=[23, None], wonder_complete=[False, None]))
    expected = [dict(kind='plot_expected', x=0, y=0, has_city_attributes=True,
                     coastal_min_one=True, wonder_complete=False,
                     valid_domains_mask=23, valid_domains=[True, True, True]),
                dict(kind='plot_expected', x=1, y=0, has_city_attributes=False,
                     coastal_min_one=False, wonder_complete=True,
                     valid_domains_mask=-1, valid_domains=[True, True, True])]
    return calculated, expected


class DomainComparisonTests(unittest.TestCase):
    def test_unknown_fallback_values_stay_signed_and_unavailable(self):
        calculated, expected = fixture()
        rows, groups, masks, wonders = compare(calculated, expected, 2, 1)
        self.assertEqual(groups['owned_raw_mask'], dict(equal=1, mismatched=0))
        self.assertEqual(groups['has_city_attributes'], dict(equal=2, mismatched=0))
        self.assertEqual(masks, {-1: 1})
        self.assertEqual(wonders, {True: 1})
        self.assertEqual(rows[1]['native_expected'], expected[1])
        self.assertEqual(len(rows[1]['unavailable']), 2)

    def test_owned_mismatch_is_retained_and_counted(self):
        calculated, expected = fixture()
        calculated['domain_reconstruction']['masks'][0] = 22
        rows, groups, _, _ = compare(calculated, expected, 2, 1)
        self.assertEqual(groups['owned_raw_mask'], dict(equal=0, mismatched=1))
        self.assertEqual(groups['owned_domain_queries'], dict(equal=0, mismatched=1))
        self.assertFalse(rows[0]['checks']['owned_raw_mask'])

    def test_missing_owned_outputs_and_invented_fallback_are_rejected(self):
        calculated, expected = fixture()
        for field, index, value in [('masks', 0, None), ('wonder_complete', 0, None),
                                    ('masks', 1, 0), ('wonder_complete', 1, False)]:
            changed = copy.deepcopy(calculated)
            changed['domain_reconstruction'][field][index] = value
            with self.assertRaises(ValueError):
                compare(changed, expected, 2, 1)

    def test_truncated_rows_wrong_order_and_inconsistent_native_answers_fail(self):
        calculated, expected = fixture()
        for changed in [expected[:1], list(reversed(expected))]:
            with self.assertRaises(ValueError):
                compare(calculated, changed, 2, 1)
        expected[0]['valid_domains'][0] = False
        with self.assertRaises(ValueError):
            compare(calculated, expected, 2, 1)

    def test_duplicate_native_json_fields_are_rejected(self):
        with self.assertRaises(ValueError):
            parse('{"valid_domains_mask":0,"valid_domains_mask":23}')


if __name__ == '__main__':
    unittest.main()
