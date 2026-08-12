import importlib.util
import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "verify-allocation-evidence.py"
SPEC = importlib.util.spec_from_file_location("allocation_evidence", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
VERIFIER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = VERIFIER
SPEC.loader.exec_module(VERIFIER)


def valid_evidence() -> dict:
    return {
        "schema_version": 2,
        "object_sizes": {"CheetahString": 32},
        "allocations": {
            "CheetahString": {
                "borrowed": {"count": 1, "bytes": 1040},
                "owned_exact": {"count": 1, "bytes": 1040},
                "owned_spare": {"count": 2, "bytes": 2064},
                "from_arc_str": {"count": 0, "bytes": 0},
                "clone": {"count": 0, "bytes": 0},
                "char": {"count": 0, "bytes": 0},
                "short_concat": {"count": 0, "bytes": 0},
            }
        },
        "invariants": {"from_arc_str_pointer_reused": True},
    }


class AllocationEvidenceTests(unittest.TestCase):
    def test_valid_contract_passes(self) -> None:
        VERIFIER.validate_evidence(valid_evidence())

    def test_allocation_regression_fails_closed(self) -> None:
        evidence = valid_evidence()
        evidence["allocations"]["CheetahString"]["short_concat"]["count"] = 1

        with self.assertRaisesRegex(VERIFIER.EvidenceError, "short_concat.count"):
            VERIFIER.validate_evidence(evidence)

    def test_missing_or_false_pointer_invariant_is_rejected(self) -> None:
        for value in (False, None):
            with self.subTest(value=value):
                evidence = deepcopy(valid_evidence())
                if value is None:
                    evidence["invariants"].pop("from_arc_str_pointer_reused")
                else:
                    evidence["invariants"]["from_arc_str_pointer_reused"] = value
                with self.assertRaises(VERIFIER.EvidenceError):
                    VERIFIER.validate_evidence(evidence)

    def test_log_parser_requires_one_evidence_record(self) -> None:
        record = json.dumps(valid_evidence(), separators=(",", ":"))
        parsed = VERIFIER.parse_log(f"noise\nSHARED_BACKING_EVIDENCE={record}\n")
        self.assertEqual(parsed, valid_evidence())

        with self.assertRaises(VERIFIER.EvidenceError):
            VERIFIER.parse_log("no evidence here")

    def test_windows_utf16_capture_is_supported(self) -> None:
        record = json.dumps(valid_evidence(), separators=(",", ":"))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.log"
            path.write_text(f"SHARED_BACKING_EVIDENCE={record}\n", encoding="utf-16")
            parsed = VERIFIER.parse_log(VERIFIER.read_log(path))

        self.assertEqual(parsed, valid_evidence())


if __name__ == "__main__":
    unittest.main()
