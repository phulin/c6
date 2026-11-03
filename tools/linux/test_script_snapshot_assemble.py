"""Opt-in static-plan fields only; no original load or execution."""
import hashlib
from pathlib import Path
import unittest
from full_manager_observer_plan_assemble import add_script_snapshot,ROOT
from script_state_snapshot import ARGUMENT_BODIES,validate_spec


class Tests(unittest.TestCase):
    def sources(self):
        source=ROOT/'tools/linux/script_state_snapshot.py'
        return {str(source):hashlib.sha256(source.read_bytes()).hexdigest()}

    def test_optin_preserves_original_fragment_and_adds_exact_layout_bodies(self):
        original=dict(live_bodies=[dict(name='own marker',module='/preload',relative_address=12,size=1,sha256='a'*64)],
            caller_suffix_frames=[dict(module='/preload',relative_pc=99)])
        added=add_script_snapshot(original,'/actual/host',self.sources())
        self.assertNotIn('script_snapshot',original);self.assertEqual(len(original['live_bodies']),1)
        validate_spec(added['script_snapshot'])
        self.assertEqual(added['caller_suffix_frames'],original['caller_suffix_frames'])
        self.assertEqual([(r['name'],r['relative_address'],r['size'],r['sha256'])
            for r in added['live_bodies'][1:]],ARGUMENT_BODIES)
        self.assertTrue(all(r['module']=='/actual/host' for r in added['live_bodies'][1:]))
        # Reapplying the same opt-in does not duplicate body gates.
        self.assertEqual(add_script_snapshot(added,'/actual/host',self.sources()),added)

    def test_unbound_or_changed_collector_refused(self):
        for sources in ({},{str(ROOT/'tools/linux/script_state_snapshot.py'):'0'*64}):
            with self.assertRaisesRegex(ValueError,'inventory'):
                add_script_snapshot(dict(live_bodies=[]),'/actual/host',sources)

    def test_conflicting_body_in_fragment_refused(self):
        _,address,size,sha=ARGUMENT_BODIES[0]
        for wrong in (dict(size=size+1,sha256=sha),dict(size=size,sha256='0'*64)):
            with self.assertRaisesRegex(ValueError,'conflicting'):
                add_script_snapshot(dict(live_bodies=[dict(module='/actual/host',relative_address=address,**wrong)]),
                    '/actual/host',self.sources())


if __name__=='__main__':unittest.main()
