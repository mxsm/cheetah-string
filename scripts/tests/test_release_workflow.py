import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class ReleaseWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )

    def test_input_crosses_shell_boundary_through_env(self):
        self.assertIn("INPUT_VERSION: ${{ inputs.version }}", self.workflow)
        self.assertNotIn('VERSION="${{ inputs.version }}"', self.workflow)

    def test_validation_precedes_tag_and_publish(self):
        package = self.workflow.index("- name: Package crate")
        tag = self.workflow.index("- name: Create tag for manual release")
        publish = self.workflow.index("- name: Publish crate to crates.io")
        self.assertLess(package, tag)
        self.assertLess(tag, publish)

    def test_default_permissions_are_read_only(self):
        self.assertRegex(self.workflow, r"permissions:\s*\n\s+contents: read")
        self.assertRegex(
            self.workflow,
            r"publish:\s*\n(?:.|\n)*?permissions:\s*\n\s+contents: write",
        )

    def test_external_actions_are_pinned(self):
        refs = re.findall(r"(?m)^\s*uses:\s*([^\s]+)", self.workflow)
        self.assertTrue(refs)
        for ref in refs:
            if ref.startswith("./"):
                continue
            self.assertRegex(ref, r"@[0-9a-f]{40}$")

    def test_publish_waits_for_validation(self):
        self.assertRegex(
            self.workflow,
            r"publish:\s*\n\s+needs:\s*\[validate\]",
        )

    def test_manual_only_trigger_avoids_duplicate_publish(self):
        self.assertIn("  workflow_dispatch:\n", self.workflow)
        self.assertNotRegex(self.workflow, r"(?m)^  push:")

    def test_artifact_digest_is_verified_before_tag(self):
        verify = self.workflow.index("- name: Verify candidate digest")
        tag = self.workflow.index("- name: Create tag for manual release")
        self.assertLess(verify, tag)


if __name__ == "__main__":
    unittest.main()
