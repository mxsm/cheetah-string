import ast
import json
import re
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "bench-results/gates/v3-score-gates.json"
RELEASE_README_PATH = ROOT / "bench-results/release/README.md"

EXPECTED_ATTESTATIONS = {
    ".github/workflows/ci.yaml": {
        "artifact": "cheetah-score-attestation-ci",
        "event": "push",
        "claims": {
            ("architecture-clone-semantics", "ci-contracts"),
            ("architecture-builder-owns-mutation", "ci-contracts"),
            ("architecture-executable-boundaries", "ci-contracts"),
            ("performance-default-search-path", "ci-contracts"),
            ("safety-typed-split", "ci-contracts"),
            ("engineering-deterministic-contracts", "ci-contracts"),
            ("engineering-versioned-perf-gate", "ci-contracts"),
            ("engineering-downstream-and-platforms", "ci-platforms"),
            ("documentation-bytes-contract", "ci-contracts"),
            ("documentation-migration-release-bundle", "ci-contracts"),
        },
    },
    ".github/workflows/performance.yml": {
        "artifact": "cheetah-score-attestation-performance",
        "event": "workflow_dispatch",
        "claims": {
            ("performance-zero-allocation-clone", "performance-final"),
            ("performance-final-geomean", "performance-final"),
            ("performance-fixed-runner-budgets", "performance-final"),
        },
    },
    ".github/workflows/safety.yml": {
        "artifact": "cheetah-score-attestation-safety",
        "event": "workflow_dispatch",
        "claims": {("safety-utf8-and-unsafe", "safety-stable")},
    },
    ".github/workflows/downstream-evidence.yml": {
        "artifact": "cheetah-score-attestation-downstream",
        "event": "workflow_dispatch",
        "claims": {
            ("engineering-downstream-and-platforms", "downstream-traffic")
        },
    },
}


def read_text(relative_path):
    return (ROOT / relative_path).read_text(encoding="utf-8")


def embedded_python_scripts(workflow):
    scripts = []
    lines = workflow.splitlines()
    index = 0
    while index < len(lines):
        if re.search(r"<<\s*'PY'\s*$", lines[index]):
            start = index + 1
            index = start
            while index < len(lines) and lines[index].strip() != "PY":
                index += 1
            if index == len(lines):
                raise AssertionError("unterminated Python heredoc in workflow")
            scripts.append(textwrap.dedent("\n".join(lines[start:index])))
        index += 1
    return scripts


def attestation_script(workflow):
    candidates = [
        script
        for script in embedded_python_scripts(workflow)
        if '"candidate_git_sha"' in script and '"claims"' in script
    ]
    if len(candidates) != 1:
        raise AssertionError(
            f"expected one attestation Python script, found {len(candidates)}"
        )
    return candidates[0]


def dict_items(node):
    if not isinstance(node, ast.Dict):
        return {}
    result = {}
    for key, value in zip(node.keys, node.values):
        if isinstance(key, ast.Constant) and isinstance(key.value, str):
            result[key.value] = value
    return result


def passed_claims(script):
    tree = ast.parse(script)
    claims = set()
    claim_ids = None

    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            if any(
                isinstance(target, ast.Name) and target.id == "claim_ids"
                for target in node.targets
            ):
                claim_ids = ast.literal_eval(node.value)

        items = dict_items(node)
        if not {"id", "claim", "status"} <= items.keys():
            continue
        claim = items["claim"]
        status = items["status"]
        if not (
            isinstance(claim, ast.Constant)
            and isinstance(claim.value, str)
            and isinstance(status, ast.Constant)
            and status.value == "passed"
        ):
            continue
        condition_id = items["id"]
        if isinstance(condition_id, ast.Constant) and isinstance(
            condition_id.value, str
        ):
            claims.add((condition_id.value, claim.value))
        elif isinstance(condition_id, ast.Name) and condition_id.id == "condition":
            if not isinstance(claim_ids, list) or not all(
                isinstance(item, str) for item in claim_ids
            ):
                raise AssertionError("dynamic CI claims require literal claim_ids")
            claims.update((item, claim.value) for item in claim_ids)

    return claims


def top_level_events(workflow):
    start_match = re.search(r"(?m)^on:\s*$", workflow)
    if start_match is None:
        raise AssertionError("workflow has no top-level on block")
    end_match = re.search(
        r"(?m)^(permissions|concurrency|env|jobs):\s*$",
        workflow[start_match.end() :],
    )
    if end_match is None:
        raise AssertionError("workflow on block has no following top-level section")
    header = workflow[
        start_match.end() : start_match.end() + end_match.start()
    ]
    return set(re.findall(r"(?m)^  ([A-Za-z_][A-Za-z0-9_-]*):", header))


class WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
        cls.conditions = cls.manifest["score"]["conditions"]

    def test_manifest_attestation_contract_is_exact(self):
        condition_ids = [condition["id"] for condition in self.conditions]
        expected_condition_ids = {
            condition_id
            for contract in EXPECTED_ATTESTATIONS.values()
            for condition_id, _ in contract["claims"]
        }
        self.assertEqual(len(condition_ids), 14)
        self.assertEqual(set(condition_ids), expected_condition_ids)
        self.assertEqual(len(condition_ids), len(set(condition_ids)))
        self.assertEqual(self.manifest["score"]["baseline"], 82)
        self.assertEqual(self.manifest["score"]["target"], 96)
        self.assertEqual(
            82 + sum(condition["points"] for condition in self.conditions), 96
        )

        actual = set()
        for condition in self.conditions:
            self.assertEqual(
                set(condition["required_evidence_kinds"]),
                {"tracked-file", "verified-ci-attestation"},
            )
            for attestation in condition["required_attestations"]:
                actual.add(
                    (
                        attestation["workflow"],
                        attestation["artifact"],
                        tuple(attestation["events"]),
                        condition["id"],
                        attestation["claim"],
                    )
                )

        expected = {
            (
                workflow_path,
                contract["artifact"],
                (contract["event"],),
                condition_id,
                claim,
            )
            for workflow_path, contract in EXPECTED_ATTESTATIONS.items()
            for condition_id, claim in contract["claims"]
        }
        self.assertEqual(actual, expected)

    def test_existing_workflows_emit_the_manifest_claims(self):
        downstream_path = ".github/workflows/downstream-evidence.yml"
        for workflow_path, contract in EXPECTED_ATTESTATIONS.items():
            if workflow_path == downstream_path:
                continue
            workflow = read_text(workflow_path)
            self.assertIn(contract["event"], top_level_events(workflow))
            self.assertEqual(
                set(
                    re.findall(
                        r"(?m)^\s+name:\s*"
                        r"(cheetah-score-attestation-[a-z0-9-]+)\s*$",
                        workflow,
                    )
                ),
                {contract["artifact"]},
            )
            self.assertIn("score-attestation.json", workflow)
            self.assertEqual(
                passed_claims(attestation_script(workflow)),
                contract["claims"],
            )

        ci = read_text(".github/workflows/ci.yaml")
        self.assertIn("if: github.event_name == 'push'", ci)
        self.assertIn("macos-15-intel", ci)
        self.assertNotIn("macos-latest", ci)

        performance = read_text(".github/workflows/performance.yml")
        self.assertIn("COMPARE_MODE: ${{ inputs.mode || 'pr' }}", performance)
        self.assertGreaterEqual(
            performance.count("if: env.COMPARE_MODE == 'final'"), 2
        )

        safety = read_text(".github/workflows/safety.yml")
        self.assertIn("if: github.event_name == 'workflow_dispatch'", safety)

    def test_action_refs_are_immutable_and_toolchains_are_explicit(self):
        for workflow_path in (
            ".github/workflows/ci.yaml",
            ".github/workflows/performance.yml",
            ".github/workflows/release.yml",
            ".github/workflows/safety.yml",
        ):
            workflow = read_text(workflow_path)
            uses = re.findall(
                r"(?m)^\s*(?:-\s+)?uses:\s*([^#\s]+)",
                workflow,
            )
            self.assertTrue(uses, workflow_path)
            for action in uses:
                if action.startswith("./") or action.startswith("docker://"):
                    continue
                self.assertRegex(
                    action,
                    r"^[^@\s]+@[0-9a-f]{40}$",
                    f"{workflow_path} has a mutable action ref: {action}",
                )

            toolchain_steps = re.findall(
                r"uses:\s*dtolnay/rust-toolchain@[0-9a-f]{40}[^\r\n]*\r?\n"
                r"\s+with:\r?\n\s+toolchain:\s*([^\s#]+)",
                workflow,
            )
            self.assertEqual(
                workflow.count("uses: dtolnay/rust-toolchain@"),
                len(toolchain_steps),
                f"{workflow_path} has an implicit Rust toolchain",
            )
            self.assertTrue(
                set(toolchain_steps) <= {"stable", "nightly", "1.75.0"}
            )

    def test_downstream_attestation_is_explicitly_external_and_pending(self):
        downstream = ".github/workflows/downstream-evidence.yml"
        self.assertFalse(
            (ROOT / downstream).exists(),
            "downstream evidence must remain externally provisioned",
        )

        readme = RELEASE_README_PATH.read_text(encoding="utf-8")
        self.assertIn("cheetah-score-attestation-downstream", readme)
        self.assertIn(f"`{downstream}`", readme)
        self.assertIn(
            "The downstream workflow is intentionally not provisioned in this "
            "repository.",
            readme,
        )
        self.assertIn(
            "`engineering-downstream-and-platforms` condition remains "
            "incomplete",
            readme,
        )
        self.assertIn("release is blocked", readme)

    def test_release_workflow_calls_fail_closed_verifier(self):
        release = read_text(".github/workflows/release.yml")
        self.assertEqual(top_level_events(release), {"workflow_dispatch"})
        self.assertIn("GITHUB_API_URL: ${{ github.api_url }}", release)
        self.assertIn("GITHUB_REPOSITORY: ${{ github.repository }}", release)
        self.assertIn("GITHUB_TOKEN: ${{ github.token }}", release)

        invocation = re.search(
            r"python3 scripts/verify-score\.py\s+"
            r"--manifest bench-results/gates/v3-score-gates\.json\s+"
            r"--release-only\s+"
            r'--github-repository "\$GITHUB_REPOSITORY"\s+'
            r"--candidate-package\s+"
            r'"\$RUNNER_TEMP/validated-release/'
            r"cheetah-string-\$\{\{ needs\.validate\.outputs\.version \}\}"
            r'\.crate"\s+'
            r"--output target/release-score\.json",
            release,
        )
        self.assertIsNotNone(invocation)
        self.assertNotIn("--evidence", release)
        self.assertLess(
            release.index(
                "Download and verify independently validated candidate crate"
            ),
            release.index("Verify live immutable 96-point evidence"),
        )

    def test_benchmark_hygiene_and_32_bit_layout_are_explicit(self):
        comprehensive = read_text("benches/comprehensive.rs")
        measured = comprehensive[
            comprehensive.index("fn bench_creation")
            : comprehensive.index("fn bench_query")
        ]
        self.assertIn("iter_batched", measured)
        self.assertNotRegex(measured, r"\bb\.iter\(")

        topic = read_text("benches/mq_topic.rs")
        self.assertIn("Throughput::Elements(1)", topic)
        self.assertNotIn(
            "Throughput::Elements(needles.len() as u64)",
            topic,
        )

        layout = read_text("tests/layout_snapshot.rs")
        pointer_32 = layout[layout.index('#[cfg(target_pointer_width = "32")]') :]
        self.assertIn("size_of::<CheetahString>(), 28", pointer_32)
        self.assertIn("size_of::<Option<CheetahString>>(), 28", pointer_32)
        self.assertNotIn("size_of::<CheetahString>(), 16", pointer_32)

    def test_raw_packed_logs_remain_local_only(self):
        packed_root = ROOT / "bench-results" / "packed-evidence"
        materialized = {
            path.relative_to(packed_root).as_posix()
            for path in packed_root.rglob("*")
            if path.is_file()
        }
        self.assertEqual(
            materialized,
            {"20260624-130416/summary.md"},
        )

        gitignore = read_text(".gitignore")
        self.assertIn("bench-results/packed-evidence/**/*", gitignore)
        self.assertIn(
            "!bench-results/packed-evidence/**/summary.md",
            gitignore,
        )

        cargo_toml = read_text("Cargo.toml")
        self.assertIn('"bench-results/packed-evidence/**"', cargo_toml)

    def test_reports_do_not_promote_diagnostic_evidence(self):
        architecture = read_text(
            "cheetah-string-architecture-performance-report.html"
        )
        optimization = read_text(
            "cheetah-string-95plus-optimization-plan.html"
        )
        self.assertIn("historical diagnostic", architecture)
        self.assertIn("当前 exact-candidate", architecture)
        self.assertIn("本轮未访问 live 下游", architecture)
        self.assertIn("PR 不运行", optimization)
        self.assertIn("当前 exact-candidate", optimization)
        self.assertNotIn("PR 60s", optimization)
        for report in (architecture, optimization):
            self.assertIn(
                "历史本地代表性合成微负载（non-scoring）",
                report,
            )
            self.assertIn(
                "local PASS 不等于 exact-release attestation",
                report,
            )
            self.assertIn("来自不同的历史本地 capture", report)
            self.assertIn(
                "没有可提交、可复算的 raw capture identity",
                report,
            )
            self.assertIn("不可横向比较、不可评分", report)
            self.assertIn(
                ".github/workflows/downstream-evidence.yml",
                report,
            )
            self.assertIn("当前未落地", report)
            self.assertIn("外部隔离环境与授权", report)

        self.assertNotIn("真实工作负载", optimization)
        self.assertIn(
            "193 tests · 163 unit/integration + 30 docs",
            optimization,
        )
        self.assertIn(
            "193 项测试（163 unit/integration + 30 doctests）",
            optimization,
        )
        self.assertTrue(
            (ROOT / "docs/adr/006-experimental-packed-boundary.md").is_file()
        )
        self.assertTrue((ROOT / "docs/stable-unsafe-audit.md").is_file())

    def test_report_links_are_relative_and_resolve(self):
        reports = (
            "cheetah-string-architecture-performance-report.html",
            "cheetah-string-95plus-optimization-plan.html",
        )
        required_links = {
            "bench-results/gates/v3-score-gates.json",
            ".github/workflows/ci.yaml",
            ".github/workflows/performance.yml",
            ".github/workflows/safety.yml",
            "bench-results/release/README.md",
            "docs/adr/006-experimental-packed-boundary.md",
            "docs/bytes-interop.md",
            "docs/migration-v2-to-v3.md",
            "docs/performance-gates.md",
            "docs/stable-unsafe-audit.md",
        }

        for report_path in reports:
            report = read_text(report_path)
            links = set(re.findall(r'href="([^"]+)"', report))
            local_paths = set()

            for link in links:
                path_text, _, fragment = link.partition("#")
                if not path_text:
                    target_path = ROOT / report_path
                else:
                    self.assertNotRegex(path_text, r"^[A-Za-z][A-Za-z0-9+.-]*:")
                    self.assertFalse(Path(path_text).is_absolute())
                    target_path = ROOT / path_text
                    local_paths.add(path_text)

                self.assertTrue(
                    target_path.is_file(),
                    f"{report_path}: unresolved link {link}",
                )

                if fragment and target_path.suffix.lower() == ".html":
                    target = target_path.read_text(encoding="utf-8")
                    self.assertRegex(
                        target,
                        rf'\bid="{re.escape(fragment)}"',
                        f"{report_path}: unresolved anchor {link}",
                    )

            self.assertTrue(
                required_links <= local_paths,
                f"{report_path}: missing key evidence links "
                f"{sorted(required_links - local_paths)}",
            )


if __name__ == "__main__":
    unittest.main()
