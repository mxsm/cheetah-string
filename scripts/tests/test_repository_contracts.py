import re
import subprocess
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


class RepositoryContractTests(unittest.TestCase):
    def test_declared_msrv_is_enforced_everywhere(self) -> None:
        with (ROOT / "Cargo.toml").open("rb") as manifest_file:
            manifest = tomllib.load(manifest_file)

        self.assertEqual(manifest["package"]["rust-version"], "1.95")
        self.assertIn("rust-1.95%2B", read("README.md"))
        self.assertIn("minimum supported Rust version is 1.95", read("README.md"))
        self.assertIn("MSRV=${1:-1.95}", read("scripts/check-msrv-package.sh"))
        self.assertIn('[string]$Msrv = "1.95"', read("scripts/check-msrv-package.ps1"))
        self.assertIn("toolchain: 1.95", read(".github/workflows/ci.yaml"))

    def test_root_lockfile_is_tracked_and_auditable(self) -> None:
        ignored = subprocess.run(
            ["git", "check-ignore", "Cargo.lock"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(ignored.returncode, 0, ignored.stdout)
        self.assertTrue((ROOT / "Cargo.lock").is_file())

    def test_unmaintained_benchmark_dependency_is_absent(self) -> None:
        checked = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", *sorted((ROOT / "benches").glob("*.rs"))]
        for path in checked:
            self.assertNotIn("smartstring", path.read_text(encoding="utf-8").lower(), path)

    def test_internal_unchecked_helpers_are_unsafe_boundaries(self) -> None:
        source = read("src/cheetah_string/construct.rs")
        for helper in (
            "from_validated_vec_unchecked",
            "from_validated_arc_vec_unchecked",
            "from_validated_bytes_unchecked",
        ):
            self.assertRegex(source, rf"unsafe fn {helper}\b")

    def test_workflow_actions_are_immutable(self) -> None:
        for workflow in sorted(WORKFLOWS.glob("*.y*ml")):
            text = workflow.read_text(encoding="utf-8")
            action_lines = [line for line in text.splitlines() if "uses:" in line]
            self.assertTrue(action_lines, workflow)
            for line in action_lines:
                self.assertRegex(line, r"^\s*uses:\s+[^\s@]+@[0-9a-f]{40}(?:\s+#.*)?$", workflow)

    def test_ci_contains_reproducible_engineering_gates(self) -> None:
        ci = read(".github/workflows/ci.yaml")
        safety = read(".github/workflows/safety.yml")
        for command in (
            "cargo clippy --all-targets --all-features -- -D warnings",
            "RUSTDOCFLAGS=\"-D warnings\" cargo doc --lib --no-deps",
            "python -m unittest discover -s scripts/tests -v",
            "cargo audit -D warnings",
            "scripts/check-msrv-package.sh 1.95",
        ):
            self.assertIn(command, ci)
        for command in (
            "cargo miri test --lib --no-default-features",
            "cargo miri test --test bytes --features bytes",
            "cargo fuzz build",
        ):
            self.assertIn(command, safety)

    def test_retired_score_governance_is_not_shipped(self) -> None:
        retired = (
            "scripts/verify-score.py",
            "scripts/tests/test_verify_score.py",
            "scripts/tests/test_workflow_contracts.py",
            "scripts/compare-benchmarks.ps1",
            "scripts/compare-benchmarks.sh",
        )
        for path in retired:
            self.assertFalse((ROOT / path).exists(), path)

    def test_docs_directory_is_not_tracked(self) -> None:
        tracked = subprocess.run(
            ["git", "ls-files", "docs"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        self.assertEqual(tracked.stdout.strip(), "")


if __name__ == "__main__":
    unittest.main()
