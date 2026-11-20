import importlib.util
import json
import re
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "instruction-counts.py"
SPEC = importlib.util.spec_from_file_location("instruction_counts", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
COUNTS = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COUNTS
SPEC.loader.exec_module(COUNTS)

CACHEGRIND_STDERR = """\
==4242== Cachegrind, a high-precision tracing profiler
==4242== Command: ./cheetah-icount eq/inline_14B 11000
==4242==
==4242== I refs:        1,234,567
"""


class InstructionCountTests(unittest.TestCase):
    def test_parses_cachegrind_summary(self) -> None:
        self.assertEqual(COUNTS.parse_irefs(CACHEGRIND_STDERR), 1_234_567)

    def test_rejects_output_without_a_summary(self) -> None:
        with self.assertRaises(ValueError):
            COUNTS.parse_irefs("==1== Command: ./cheetah-icount\n")

    def test_per_iteration_cancels_fixed_costs(self) -> None:
        # 500_000 start-up and setup instructions plus 40 per iteration.
        low = 500_000 + 40 * 1_000
        high = 500_000 + 40 * 11_000
        self.assertEqual(COUNTS.per_iteration(low, high, 1_000, 11_000), 40.0)

    def test_per_iteration_rejects_a_falling_count(self) -> None:
        with self.assertRaises(ValueError):
            COUNTS.per_iteration(2_000, 1_000, 1_000, 11_000)

    def test_regression_needs_relative_and_absolute_growth(self) -> None:
        base = {"small": 10.0, "large": 1_000.0, "faster": 50.0}
        head = {"small": 11.5, "large": 1_030.0, "faster": 40.0}
        rows = {row.scenario: row for row in COUNTS.compare(base, head, 0.02, 2.0)}
        # +15% but only 1.5 instructions: below the absolute floor.
        self.assertFalse(rows["small"].regressed)
        # +3% and 30 instructions: a regression.
        self.assertTrue(rows["large"].regressed)
        self.assertFalse(rows["faster"].regressed)
        self.assertAlmostEqual(rows["faster"].change, -0.2)

    def test_missing_base_reports_head_only(self) -> None:
        rows = COUNTS.compare(None, {"eq/inline_14B": 12.0}, 0.02, 2.0)
        self.assertEqual(len(rows), 1)
        self.assertIsNone(rows[0].base)
        self.assertFalse(rows[0].regressed)
        report = COUNTS.render(rows, 0.02, 2.0, "")
        self.assertIn("| eq/inline_14B | - | 12.0 | - |", report)
        self.assertIn("No regressions.", report)

    def test_report_names_regressions(self) -> None:
        rows = COUNTS.compare({"find": 100.0}, {"find": 110.0}, 0.02, 2.0)
        report = COUNTS.render(rows, 0.02, 2.0, "Base `a`, head `b`.")
        self.assertIn("**+10.0% regression**", report)
        self.assertIn("**1 regression(s):** find", report)

    def test_only_harness_compile_errors_allow_a_head_only_report(self) -> None:
        def message(target: str, level: str, rendered: str) -> str:
            return json.dumps(
                {
                    "reason": "compiler-message",
                    "target": {"kind": ["bin"], "name": target},
                    "message": {"level": level, "message": rendered, "rendered": rendered},
                }
            )

        harness_error = message("cheetah-icount", "error", "error[E0599]: no method `new_api`\n")
        crate_error = message("cheetah_string", "error", "error[E0308]: mismatched types\n")
        harness_warning = message("cheetah-icount", "warning", "warning: unused variable\n")
        output = "\n".join(
            [
                "not json",
                json.dumps({"reason": "build-finished", "success": False}),
                crate_error,
                harness_warning,
                harness_error,
            ]
        )
        self.assertEqual(COUNTS.harness_errors(output), ["error[E0599]: no method `new_api`\n"])
        # A failure in the crate or a dependency is not a harness incompatibility.
        self.assertEqual(COUNTS.harness_errors("\n".join([crate_error, harness_warning])), [])
        self.assertEqual(COUNTS.harness_errors(""), [])

    def test_harness_lists_every_scenario_once(self) -> None:
        source = (ROOT / COUNTS.HARNESS).read_text(encoding="utf-8")
        table = source[source.index("const SCENARIOS"):]
        names = re.findall(r'\(\s*"([^"]+)",\s*\w+\s*\)', table)
        self.assertGreater(len(names), 10)
        self.assertEqual(len(names), len(set(names)))


if __name__ == "__main__":
    unittest.main()
