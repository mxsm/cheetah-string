from __future__ import annotations

import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
import math
import os
import sys
import tarfile
import tempfile
import threading
import unittest
import urllib.parse
import urllib.request
import zipfile
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from unittest import mock


sys.dont_write_bytecode = True
VERIFIER_PATH = Path(__file__).resolve().parents[1] / "verify-score.py"
PROJECT_ROOT = VERIFIER_PATH.parents[1]
CURRENT_MANIFEST_PATH = (
    PROJECT_ROOT / "bench-results" / "gates" / "v3-score-gates.json"
)
SPEC = importlib.util.spec_from_file_location("cheetah_score_verifier", VERIFIER_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load verifier at {VERIFIER_PATH}")
verifier = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = verifier
SPEC.loader.exec_module(verifier)


HEAD_SHA = "1" * 40
BASE_SHA = "2" * 40
REPOSITORY = "acme/cheetah"
MANIFEST_ID = "test-score-gates-v1"
TRACKED_CONTENT = b"[package]\nname = \"cheetah-string\"\n"
COMPARISON_IDS = (
    "performance-zero-allocation-clone",
    "performance-final-geomean",
    "performance-fixed-runner-budgets",
)


def json_bytes(value: dict[str, Any]) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def make_zip(files: dict[str, bytes]) -> bytes:
    target = io.BytesIO()
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in files.items():
            archive.writestr(name, content)
    return target.getvalue()


def replace_zip_json(
    archive_bytes: bytes,
    filename: str,
    value: dict[str, Any],
) -> bytes:
    return replace_zip_file(archive_bytes, filename, json_bytes(value))


def replace_zip_file(
    archive_bytes: bytes,
    filename: str,
    value: bytes,
) -> bytes:
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        files = {
            info.filename: archive.read(info)
            for info in archive.infolist()
            if not info.is_dir()
        }
    files[filename] = value
    return make_zip(files)


def read_zip_json(archive_bytes: bytes, filename: str) -> dict[str, Any]:
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        value = json.loads(archive.read(filename))
    if not isinstance(value, dict):
        raise AssertionError(f"{filename} fixture must be an object")
    return value


def make_crate_package(
    crate_name: str,
    crate_version: str,
    candidate_git_sha: str,
    readme: bytes = b"exact candidate fixture\n",
) -> bytes:
    vcs_info = json_bytes(
        {
            "git": {"sha1": candidate_git_sha},
            "path_in_vcs": "",
        }
    )
    target = io.BytesIO()
    root = f"{crate_name}-{crate_version}"
    with tarfile.open(fileobj=target, mode="w:gz") as archive:
        vcs_member = tarfile.TarInfo(f"{root}/.cargo_vcs_info.json")
        vcs_member.size = len(vcs_info)
        archive.addfile(vcs_member, io.BytesIO(vcs_info))
        readme_member = tarfile.TarInfo(f"{root}/README.md")
        readme_member.size = len(readme)
        archive.addfile(readme_member, io.BytesIO(readme))
    return target.getvalue()


def fixture_round_metrics(samples: list[dict[str, int]]) -> dict[str, float]:
    total_operations = sum(sample["operations"] for sample in samples)
    ordered_latency = sorted(sample["latency_ns"] for sample in samples)

    def percentile(value: float) -> int:
        index = max(0, math.ceil(value * len(ordered_latency)) - 1)
        return ordered_latency[index]

    return {
        "throughput_ops_per_second": (
            total_operations * 1_000_000_000.0
            / sum(sample["elapsed_ns"] for sample in samples)
        ),
        "latency_p50_ms": percentile(0.50) / 1_000_000.0,
        "latency_p95_ms": percentile(0.95) / 1_000_000.0,
        "latency_p99_ms": percentile(0.99) / 1_000_000.0,
        "cpu_time_per_operation_ns": (
            sum(sample["cpu_time_ns"] for sample in samples)
            / total_operations
        ),
        "rss_peak_bytes": float(
            max(sample["rss_peak_bytes"] for sample in samples)
        ),
        "allocations_per_operation": (
            sum(sample["allocations"] for sample in samples)
            / total_operations
        ),
    }


def make_downstream_files(
    manifest: dict[str, Any],
    *,
    attestation_run_id: int = 9002,
) -> dict[str, bytes]:
    policy = manifest["downstream_traffic_policy"]
    crate_policy = policy["crate"]
    package = make_crate_package(
        crate_policy["name"],
        crate_policy["version"],
        HEAD_SHA,
    )
    files: dict[str, bytes] = {
        crate_policy["package_filename"]: package,
    }
    input_artifacts: dict[str, dict[str, str]] = {}
    for index, (role, filename) in enumerate(
        sorted(policy["required_input_artifacts"].items()),
        start=1,
    ):
        content = json_bytes(
            {
                "schema_version": 1,
                "role": role,
                "fixture": index,
            }
        )
        files[filename] = content
        input_artifacts[role] = {
            "filename": filename,
            "sha256": hashlib.sha256(content).hexdigest(),
        }

    rounds: list[dict[str, Any]] = []
    measurement_policy = policy["measurement"]
    sample_count = measurement_policy["minimum_samples_per_round"]
    input_sha256 = {
        role: descriptor["sha256"]
        for role, descriptor in input_artifacts.items()
    }
    for index in range(1, measurement_policy["minimum_rounds"] + 1):
        filename = measurement_policy["round_artifact_filename_pattern"].format(
            round=index
        )
        baseline_samples: list[dict[str, int]] = []
        candidate_samples: list[dict[str, int]] = []
        for sequence in range(1, sample_count + 1):
            if sequence <= 50:
                latency_ns = 10_000_000
            elif sequence <= 95:
                latency_ns = 20_000_000
            else:
                latency_ns = 30_000_000
            baseline_samples.append(
                {
                    "sequence": sequence,
                    "operations": 1,
                    "elapsed_ns": 1_000_000,
                    "latency_ns": latency_ns,
                    "cpu_time_ns": 100,
                    "rss_peak_bytes": 1000,
                    "allocations": 10,
                }
            )
            candidate_samples.append(
                {
                    "sequence": sequence,
                    "operations": 1,
                    "elapsed_ns": (
                        1_010_000 + (index if sequence == sample_count else 0)
                    ),
                    "latency_ns": latency_ns * 101 // 100,
                    "cpu_time_ns": 101,
                    "rss_peak_bytes": 1010,
                    "allocations": 11 if sequence <= 10 else 10,
                }
            )
        started_at = 1_800_000_000_000 + (index - 1) * 600_000
        finished_at = (
            started_at + measurement_policy["minimum_duration_seconds"] * 1000
        )
        content = json_bytes(
            {
                "schema_version": measurement_policy["round_schema_version"],
                "round_id": f"r{index}",
                "capture_id": hashlib.sha256(
                    f"fixture-capture-{index}".encode()
                ).hexdigest(),
                "attestation_run_id": attestation_run_id,
                "candidate_git_sha": HEAD_SHA,
                "candidate_package_sha256": hashlib.sha256(package).hexdigest(),
                "downstream": policy["downstream"],
                "workload_id": policy["workload_id"],
                "input_sha256": input_sha256,
                "started_at_unix_ms": started_at,
                "finished_at_unix_ms": finished_at,
                "baseline_samples": baseline_samples,
                "candidate_samples": candidate_samples,
            }
        )
        files[filename] = content
        rounds.append(
            {
                "id": f"r{index}",
                "sample_count": sample_count,
                "raw_artifact": {
                    "filename": filename,
                    "sha256": hashlib.sha256(content).hexdigest(),
                },
                "metrics": {
                    "baseline": fixture_round_metrics(baseline_samples),
                    "candidate": fixture_round_metrics(candidate_samples),
                },
            }
        )

    evidence = {
        "schema_version": 2,
        "policy_id": policy["policy_id"],
        "manifest_id": manifest["manifest_id"],
        "candidate_git_sha": HEAD_SHA,
        "crate": {
            "name": crate_policy["name"],
            "version": crate_policy["version"],
            "package_filename": crate_policy["package_filename"],
            "package_sha256": hashlib.sha256(package).hexdigest(),
            "candidate_git_sha": HEAD_SHA,
        },
        "downstream": {
            "repository": policy["downstream"]["repository"],
            "source_git_sha": policy["downstream"]["source_git_sha"],
        },
        "workload_id": policy["workload_id"],
        "input_artifacts": input_artifacts,
        "crate_feature_matrix": [
            {"profile": profile, "status": "passed"}
            for profile in policy["package_matrix"][
                "required_crate_feature_profiles"
            ]
        ],
        "package_matrix": {
            "workspace": [
                {
                    "package": f"workspace-package-{index:02d}",
                    "features": ["all-features"],
                    "status": "passed",
                }
                for index in range(
                    1,
                    policy["package_matrix"]["minimum_workspace_packages"] + 1,
                )
            ],
            "standalone": [
                {
                    "package": f"standalone-package-{index:02d}",
                    "features": ["all-features"],
                    "status": "passed",
                }
                for index in range(
                    1,
                    policy["package_matrix"]["minimum_standalone_packages"] + 1,
                )
            ],
        },
        "measurement": {
            "warmup_seconds": measurement_policy["minimum_warmup_seconds"],
            "duration_seconds": measurement_policy["minimum_duration_seconds"],
            "rounds": rounds,
        },
    }
    files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
    return files


def make_ci_platform_files(
    manifest: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, bytes]]:
    policy = manifest["ci_platform_policy"]
    checks = {entry["id"]: entry for entry in policy["required_checks"]}
    files: dict[str, bytes] = {}
    platform_files: list[dict[str, Any]] = []
    for platform in policy["platforms"]:
        matrix_os = platform["matrix_os"]
        header = (
            f"candidate_git_sha={HEAD_SHA}\n"
            f"matrix_os={matrix_os}\n"
            f"runner_os={platform['runner_os']}\n"
            f"runner_arch={platform['runner_arch']}\n"
            "rustc 1.90.0 (fixture 2026-07-26)\n"
            f"host: {platform['rustc_host']}\n"
        )
        source_logs: dict[str, bytes] = {}
        score_logs: dict[str, dict[str, str]] = {}
        for check_id, check in checks.items():
            raw = (
                header
                + f"{check['passing_marker']}\n"
                + f"platform_check={check_id}:passed\n"
            ).encode()
            source_filename = check["log_filename"]
            source_logs[source_filename] = raw
            score_filename = f"platform-{matrix_os}-{source_filename}"
            files[score_filename] = raw
            score_logs[check_id] = {
                "filename": score_filename,
                "sha256": hashlib.sha256(raw).hexdigest(),
            }
        descriptor = {
            "schema_version": policy["descriptor_schema_version"],
            "candidate_git_sha": HEAD_SHA,
            "matrix_os": matrix_os,
            "runner_os": platform["runner_os"],
            "runner_arch": platform["runner_arch"],
            "rustc": (
                "rustc 1.90.0 (fixture 2026-07-26)\n"
                f"host: {platform['rustc_host']}"
            ),
            "checks": {check_id: "passed" for check_id in checks},
            "log_sha256": {
                filename: hashlib.sha256(raw).hexdigest()
                for filename, raw in source_logs.items()
            },
        }
        descriptor_filename = f"platform-{matrix_os}.json"
        descriptor_raw = json_bytes(descriptor)
        files[descriptor_filename] = descriptor_raw
        platform_files.append(
            {
                "matrix_os": matrix_os,
                "descriptor": {
                    "filename": descriptor_filename,
                    "sha256": hashlib.sha256(descriptor_raw).hexdigest(),
                },
                "logs": score_logs,
            }
        )
    return (
        {
            "ci_platform_policy_id": policy["policy_id"],
            "workflow_needs": policy["required_workflow_needs"],
            "platform_files": platform_files,
        },
        files,
    )


def make_safety_files(
    manifest: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, bytes]]:
    policy = manifest["safety_stable_policy"]
    files: dict[str, bytes] = {}
    safety_files: list[dict[str, Any]] = []
    for required in policy["required_logs"]:
        parts = [
            f"candidate_git_sha={HEAD_SHA}",
            f"target={required['target']}",
        ]
        if required["kind"] == "test":
            parts.append(required["test_marker"])
        else:
            parts.append(f"max_total_time={policy['minimum_fuzz_seconds']}")
        parts.append(policy["passing_marker"])
        raw = ("\n".join(parts) + "\n").encode()
        files[required["filename"]] = raw
        descriptor: dict[str, Any] = {
            "id": required["id"],
            "kind": required["kind"],
            "filename": required["filename"],
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
        if required["kind"] == "fuzz":
            descriptor["max_total_time_seconds"] = policy[
                "minimum_fuzz_seconds"
            ]
        safety_files.append(descriptor)
    return (
        {
            "safety_stable_policy_id": policy["policy_id"],
            "workflow_needs": policy["required_workflow_needs"],
            "safety_files": safety_files,
        },
        files,
    )


def with_score_attestation(
    fields: dict[str, Any],
    files: dict[str, bytes],
    condition_id: str,
    claim: str,
) -> dict[str, bytes]:
    result = dict(files)
    attestation = {
        "schema_version": 1,
        "candidate_git_sha": HEAD_SHA,
        "claims": [
            {
                "id": condition_id,
                "claim": claim,
                "status": "passed",
            }
        ],
    }
    attestation.update(fields)
    result["score-attestation.json"] = json_bytes(attestation)
    return result


def set_artifact_archive(artifact: ArtifactSpec, archive: bytes) -> None:
    artifact.archive = archive
    artifact.download = archive
    artifact.digest = hashlib.sha256(archive).hexdigest()


def make_comparison_result(
    mode: str,
    *,
    base_git_sha: str = BASE_SHA,
    schema_version: int = 2,
    metadata_compatible: bool = True,
    performance_verdict: str = "pass",
    statuses: dict[str, str] | None = None,
) -> dict[str, Any]:
    if statuses is None:
        statuses = {
            condition_id: (
                "final-comparison-required"
                if mode == "pr"
                and condition_id == "performance-final-geomean"
                else "passed"
            )
            for condition_id in COMPARISON_IDS
        }
    return {
        "schema_version": schema_version,
        "manifest_id": MANIFEST_ID,
        "mode": mode,
        "base_git_sha": base_git_sha,
        "head_git_sha": HEAD_SHA,
        "metadata_compatible": metadata_compatible,
        "performance_verdict": performance_verdict,
        "score_ledger": [
            {"id": condition_id, "status": statuses[condition_id]}
            for condition_id in COMPARISON_IDS
        ],
    }


@dataclass
class ArtifactSpec:
    artifact_id: int
    run_id: int
    workflow: str
    event: str
    archive: bytes
    digest: str
    download: bytes
    run_status: str = "completed"
    run_conclusion: str = "success"
    fake_download_url: str | None = None


class MockGitHub:
    def __init__(self, artifacts: dict[str, ArtifactSpec]) -> None:
        self.artifacts = artifacts
        self.artifacts_by_id = {
            artifact.artifact_id: (name, artifact)
            for name, artifact in artifacts.items()
        }
        self.artifacts_by_run_id = {
            artifact.run_id: artifact for artifact in artifacts.values()
        }
        self.requested_artifact_names: set[str] = set()
        self.server: ThreadingHTTPServer | None = None
        self.thread: threading.Thread | None = None
        self.api_url = ""

    def __enter__(self) -> "MockGitHub":
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, format: str, *args: object) -> None:
                del format, args

            def send_json(self, value: dict[str, Any], status: int = 200) -> None:
                body = json_bytes(value)
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def send_archive(self, value: bytes) -> None:
                self.send_response(200)
                self.send_header("Content-Type", "application/zip")
                self.send_header("Content-Length", str(len(value)))
                self.end_headers()
                self.wfile.write(value)

            def do_GET(self) -> None:
                if self.headers.get("Authorization") != "Bearer test-token":
                    self.send_json({"message": "unauthorized"}, status=401)
                    return
                parsed = urllib.parse.urlsplit(self.path)
                route = parsed.path
                repository_path = "/repos/acme/cheetah"
                if route == f"{repository_path}/actions/artifacts":
                    query = urllib.parse.parse_qs(parsed.query)
                    names = query.get("name", [])
                    if len(names) != 1 or names[0] not in owner.artifacts:
                        self.send_json({"total_count": 0, "artifacts": []})
                        return
                    name = names[0]
                    owner.requested_artifact_names.add(name)
                    artifact = owner.artifacts[name]
                    self.send_json(
                        {
                            "total_count": 1,
                            "artifacts": [
                                {
                                    "id": artifact.artifact_id,
                                    "name": name,
                                    "expired": False,
                                    "digest": f"sha256:{artifact.digest}",
                                    "workflow_run": {
                                        "id": artifact.run_id,
                                        "head_sha": HEAD_SHA,
                                    },
                                }
                            ],
                        }
                    )
                    return

                artifact_prefix = f"{repository_path}/actions/artifacts/"
                if route.startswith(artifact_prefix):
                    remainder = route[len(artifact_prefix) :]
                    is_zip = remainder.endswith("/zip")
                    raw_id = remainder[:-4] if is_zip else remainder
                    try:
                        artifact_id = int(raw_id)
                    except ValueError:
                        self.send_json({"message": "not found"}, status=404)
                        return
                    entry = owner.artifacts_by_id.get(artifact_id)
                    if entry is None:
                        self.send_json({"message": "not found"}, status=404)
                        return
                    name, artifact = entry
                    if is_zip:
                        self.send_archive(artifact.download)
                        return
                    download_url = (
                        artifact.fake_download_url
                        if artifact.fake_download_url is not None
                        else (
                            f"{owner.api_url}{repository_path}/actions/"
                            f"artifacts/{artifact_id}/zip"
                        )
                    )
                    self.send_json(
                        {
                            "id": artifact_id,
                            "name": name,
                            "expired": False,
                            "digest": f"sha256:{artifact.digest}",
                            "archive_download_url": download_url,
                            "workflow_run": {
                                "id": artifact.run_id,
                                "head_sha": HEAD_SHA,
                            },
                        }
                    )
                    return

                run_prefix = f"{repository_path}/actions/runs/"
                if route.startswith(run_prefix):
                    try:
                        run_id = int(route[len(run_prefix) :])
                    except ValueError:
                        self.send_json({"message": "not found"}, status=404)
                        return
                    artifact = owner.artifacts_by_run_id.get(run_id)
                    if artifact is None:
                        self.send_json({"message": "not found"}, status=404)
                        return
                    self.send_json(
                        {
                            "id": run_id,
                            "status": artifact.run_status,
                            "conclusion": artifact.run_conclusion,
                            "head_sha": HEAD_SHA,
                            "path": artifact.workflow,
                            "event": artifact.event,
                            "repository": {"full_name": REPOSITORY},
                        }
                    )
                    return
                self.send_json({"message": "not found"}, status=404)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        host, port = self.server.server_address
        self.api_url = f"http://{host}:{port}"
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *exc_info: object) -> None:
        del exc_info
        if self.server is not None:
            self.server.shutdown()
            self.server.server_close()
        if self.thread is not None:
            self.thread.join(timeout=5)


def make_fixture() -> tuple[dict[str, Any], dict[str, ArtifactSpec]]:
    release_groups = {
        "score-ci": {
            "ids": [f"release-{index:02d}" for index in range(1, 5)],
            "claim": "ci-contracts",
            "workflow": ".github/workflows/ci.yaml",
            "event": "push",
            "events": ["push", "workflow_dispatch"],
        },
        "score-safety": {
            "ids": [f"release-{index:02d}" for index in range(5, 8)],
            "claim": "safety-suite",
            "workflow": ".github/workflows/safety.yml",
            "event": "schedule",
            "events": ["schedule", "workflow_dispatch"],
        },
        "score-platform": {
            "ids": [f"release-{index:02d}" for index in range(8, 12)],
            "claim": "platform-suite",
            "workflow": ".github/workflows/platform.yml",
            "event": "workflow_dispatch",
            "events": ["workflow_dispatch"],
        },
        "score-performance": {
            "ids": list(COMPARISON_IDS),
            "claim": "performance-final",
            "workflow": ".github/workflows/performance.yml",
            "event": "workflow_dispatch",
            "events": ["workflow_dispatch"],
        },
    }

    conditions: list[dict[str, Any]] = []
    artifacts: dict[str, ArtifactSpec] = {}
    for artifact_index, (artifact_name, group) in enumerate(
        release_groups.items(),
        start=1,
    ):
        claim_entries = [
            {
                "id": condition_id,
                "claim": group["claim"],
                "status": "passed",
            }
            for condition_id in group["ids"]
        ]
        files = {
            "score-attestation.json": json_bytes(
                {
                    "schema_version": 1,
                    "candidate_git_sha": HEAD_SHA,
                    "claims": claim_entries,
                }
            )
        }
        if artifact_name == "score-performance":
            comparison = {
                "schema_version": 2,
                "manifest_id": MANIFEST_ID,
                "mode": "final",
                "head_git_sha": HEAD_SHA,
                "base_git_sha": BASE_SHA,
                "metadata_compatible": True,
                "performance_verdict": "pass",
                "score_ledger": [
                    {"id": condition_id, "status": "passed"}
                    for condition_id in COMPARISON_IDS
                ],
            }
            files["comparison-r1.json"] = json_bytes(comparison)
            files["comparison-r2.json"] = json_bytes(comparison)
        archive = make_zip(files)
        artifacts[artifact_name] = ArtifactSpec(
            artifact_id=1000 + artifact_index,
            run_id=2000 + artifact_index,
            workflow=str(group["workflow"]),
            event=str(group["event"]),
            archive=archive,
            digest=hashlib.sha256(archive).hexdigest(),
            download=archive,
        )
        for condition_id in group["ids"]:
            assessment = (
                "comparison"
                if condition_id in COMPARISON_IDS
                else "release-evidence"
            )
            conditions.append(
                {
                    "id": condition_id,
                    "dimension": "test",
                    "points": 1,
                    "assessment": assessment,
                    "required_evidence_kinds": [
                        "tracked-file",
                        "verified-ci-attestation",
                    ],
                    "evidence": ["Cargo.toml"],
                    "required_attestations": [
                        {
                            "claim": group["claim"],
                            "workflow": group["workflow"],
                            "artifact": artifact_name,
                            "events": group["events"],
                        }
                    ],
                }
            )

    manifest = {
        "schema_version": 1,
        "manifest_id": MANIFEST_ID,
        "frozen_baseline": {"git_sha": BASE_SHA},
        "score": {
            "baseline": 82,
            "target": 96,
            "conditions": conditions,
        },
    }
    return manifest, artifacts


def make_artifacts_for_manifest(
    manifest: dict[str, Any],
    *,
    workflow_ref_suffix: str = "",
) -> dict[str, ArtifactSpec]:
    grouped: dict[str, dict[str, Any]] = {}
    for condition in manifest["score"]["conditions"]:
        for requirement in condition["required_attestations"]:
            artifact_name = requirement["artifact"]
            group = grouped.setdefault(
                artifact_name,
                {
                    "workflow": requirement["workflow"],
                    "event": requirement["events"][0],
                    "claims": [],
                },
            )
            if group["workflow"] != requirement["workflow"]:
                raise AssertionError(
                    f"artifact {artifact_name} is shared across workflow paths"
                )
            if group["event"] not in requirement["events"]:
                raise AssertionError(
                    f"artifact {artifact_name} has incompatible allowed events"
                )
            group["claims"].append(
                {
                    "id": condition["id"],
                    "claim": requirement["claim"],
                    "status": "passed",
                }
            )

    comparison_ids = [
        condition["id"]
        for condition in manifest["score"]["conditions"]
        if condition["assessment"] == "comparison"
    ]
    base_sha = manifest["frozen_baseline"]["git_sha"]
    artifacts: dict[str, ArtifactSpec] = {}
    for artifact_index, (artifact_name, group) in enumerate(
        grouped.items(),
        start=1,
    ):
        artifact_run_id = 4000 + artifact_index
        attestation: dict[str, Any] = {
            "schema_version": 1,
            "candidate_git_sha": HEAD_SHA,
            "claims": group["claims"],
        }
        files: dict[str, bytes] = {}
        if any(
            claim["claim"] == "performance-final"
            for claim in group["claims"]
        ):
            comparison = {
                "schema_version": 2,
                "manifest_id": manifest["manifest_id"],
                "mode": "final",
                "head_git_sha": HEAD_SHA,
                "base_git_sha": base_sha,
                "metadata_compatible": True,
                "performance_verdict": "pass",
                "score_ledger": [
                    {"id": condition_id, "status": "passed"}
                    for condition_id in comparison_ids
                ],
            }
            files["comparison-r1.json"] = json_bytes(comparison)
            files["comparison-r2.json"] = json_bytes(comparison)
        if any(
            claim["claim"] == verifier.DOWNSTREAM_TRAFFIC_CLAIM
            for claim in group["claims"]
        ):
            files.update(
                make_downstream_files(
                    manifest,
                    attestation_run_id=artifact_run_id,
                )
            )
        if any(
            claim["claim"] == verifier.CI_PLATFORMS_CLAIM
            for claim in group["claims"]
        ):
            fields, platform_files = make_ci_platform_files(manifest)
            attestation.update(fields)
            files.update(platform_files)
        if any(
            claim["claim"] == verifier.SAFETY_STABLE_CLAIM
            for claim in group["claims"]
        ):
            fields, safety_files = make_safety_files(manifest)
            attestation.update(fields)
            files.update(safety_files)
        files["score-attestation.json"] = json_bytes(attestation)
        archive = make_zip(files)
        artifacts[artifact_name] = ArtifactSpec(
            artifact_id=3000 + artifact_index,
            run_id=artifact_run_id,
            workflow=str(group["workflow"]) + workflow_ref_suffix,
            event=str(group["event"]),
            archive=archive,
            digest=hashlib.sha256(archive).hexdigest(),
            download=archive,
        )
    return artifacts


class VerifyScoreTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        self.manifest, self.artifacts = make_fixture()
        self.manifest_path = self.root / "manifest.json"
        self.manifest_path.write_text(
            json.dumps(self.manifest),
            encoding="utf-8",
        )
        self.real_validate_release_manifest = verifier.validate_release_manifest
        release_manifest_patch = mock.patch.object(
            verifier,
            "validate_release_manifest",
            return_value="f" * 64,
        )
        release_manifest_patch.start()
        self.addCleanup(release_manifest_patch.stop)

    def release_args(self) -> argparse.Namespace:
        return argparse.Namespace(
            manifest=self.manifest_path,
            comparison=None,
            mode="pr",
            release_only=True,
            github_repository=REPOSITORY,
            candidate_package=self.root / "unused-generic-fixture.crate",
            output=self.root / "result.json",
        )

    def write_comparison(
        self,
        filename: str,
        value: dict[str, Any],
    ) -> Path:
        path = self.root / filename
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def comparison_args(
        self,
        comparison: Path,
        mode: str,
    ) -> argparse.Namespace:
        return argparse.Namespace(
            manifest=self.manifest_path,
            comparison=comparison,
            mode=mode,
            release_only=False,
            github_repository=None,
            candidate_package=None,
            output=self.root / "result.json",
        )

    def write_candidate_package_from_artifacts(
        self,
        manifest: dict[str, Any],
        artifacts: dict[str, ArtifactSpec],
    ) -> Path:
        policy = manifest["downstream_traffic_policy"]
        package_filename = policy["crate"]["package_filename"]
        artifact_name = next(
            requirement["artifact"]
            for condition in manifest["score"]["conditions"]
            for requirement in condition["required_attestations"]
            if requirement["claim"] == verifier.DOWNSTREAM_TRAFFIC_CLAIM
        )
        with zipfile.ZipFile(io.BytesIO(artifacts[artifact_name].archive)) as archive:
            package = archive.read(package_filename)
        path = self.root / package_filename
        path.write_bytes(package)
        return path

    def verify_comparison_value(
        self,
        value: dict[str, Any],
        mode: str,
        filename: str,
    ) -> tuple[dict[str, Any], bool]:
        path = self.write_comparison(filename, value)
        with mock.patch.object(verifier, "git_text", return_value=HEAD_SHA):
            return verifier.verify(self.comparison_args(path, mode))

    @staticmethod
    def fake_git_bytes(*args: str) -> bytes:
        if len(args) == 2 and args[0] == "show":
            expected = f"{HEAD_SHA}:Cargo.toml"
            if args[1] != expected:
                raise AssertionError(f"unexpected git object request: {args}")
            return TRACKED_CONTENT
        raise AssertionError(f"unexpected git invocation: {args}")

    def git_mocks(self) -> tuple[mock._patch[Any], mock._patch[Any]]:
        return (
            mock.patch.object(verifier, "git_text", return_value=HEAD_SHA),
            mock.patch.object(
                verifier,
                "git_bytes",
                side_effect=self.fake_git_bytes,
            ),
        )

    def verify_current_release(
        self,
        manifest: dict[str, Any],
        artifacts: dict[str, ArtifactSpec],
    ) -> tuple[dict[str, Any], bool]:
        manifest_path = self.root / "current-manifest.json"
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        args = argparse.Namespace(
            manifest=manifest_path,
            comparison=None,
            mode="pr",
            release_only=True,
            github_repository=REPOSITORY,
            candidate_package=self.write_candidate_package_from_artifacts(
                manifest,
                artifacts,
            ),
            output=self.root / "current-result.json",
        )

        def current_git_bytes(*git_args: str) -> bytes:
            if (
                len(git_args) == 2
                and git_args[0] == "show"
                and git_args[1].startswith(f"{HEAD_SHA}:")
            ):
                return f"tracked:{git_args[1]}".encode()
            raise AssertionError(f"unexpected git invocation: {git_args}")

        with MockGitHub(artifacts) as github:
            with (
                mock.patch.object(verifier, "git_text", return_value=HEAD_SHA),
                mock.patch.object(
                    verifier,
                    "git_bytes",
                    side_effect=current_git_bytes,
                ),
                mock.patch.dict(
                    os.environ,
                    {
                        "GITHUB_TOKEN": "test-token",
                        "GITHUB_API_URL": github.api_url,
                    },
                    clear=False,
                ),
            ):
                return verifier.verify(args)

    @staticmethod
    def validate_downstream_files(
        manifest: dict[str, Any],
        files: dict[str, bytes],
        candidate_package: bytes | None = None,
    ) -> dict[str, Any]:
        artifact = verifier.VerifiedArtifact(
            name="cheetah-score-attestation-downstream",
            artifact_id=9001,
            digest="a" * 64,
            run_id=9002,
            run_path=".github/workflows/downstream-evidence.yml",
            run_event="workflow_dispatch",
            files=files,
            claims=frozenset(
                {
                    (
                        "engineering-downstream-and-platforms",
                        verifier.DOWNSTREAM_TRAFFIC_CLAIM,
                    )
                }
            ),
        )
        policy = verifier.validate_downstream_policy(manifest)
        if candidate_package is None:
            candidate_package = files[policy["crate"]["package_filename"]]
        return verifier.validate_downstream_traffic(
            artifact,
            manifest,
            policy,
            HEAD_SHA,
            hashlib.sha256(candidate_package).hexdigest(),
        )

    @staticmethod
    def validate_ci_platform_files(
        manifest: dict[str, Any],
        files: dict[str, bytes],
    ) -> dict[str, Any]:
        artifact = verifier.VerifiedArtifact(
            name="cheetah-score-attestation-ci",
            artifact_id=9101,
            digest="a" * 64,
            run_id=9102,
            run_path=".github/workflows/ci.yaml",
            run_event="push",
            files=files,
            claims=frozenset(
                {
                    (
                        "engineering-downstream-and-platforms",
                        verifier.CI_PLATFORMS_CLAIM,
                    )
                }
            ),
        )
        policy = verifier.validate_ci_platform_policy(manifest)
        return verifier.validate_ci_platforms(
            artifact,
            policy,
            HEAD_SHA,
        )

    @staticmethod
    def validate_safety_files(
        manifest: dict[str, Any],
        files: dict[str, bytes],
    ) -> dict[str, Any]:
        artifact = verifier.VerifiedArtifact(
            name="cheetah-score-attestation-safety",
            artifact_id=9201,
            digest="a" * 64,
            run_id=9202,
            run_path=".github/workflows/safety.yml",
            run_event="workflow_dispatch",
            files=files,
            claims=frozenset(
                {
                    (
                        "safety-utf8-and-unsafe",
                        verifier.SAFETY_STABLE_CLAIM,
                    )
                }
            ),
        )
        policy = verifier.validate_safety_stable_policy(manifest)
        return verifier.validate_safety_stable(
            artifact,
            policy,
            HEAD_SHA,
        )

    def test_release_requires_github_token(self) -> None:
        git_text_mock, git_bytes_mock = self.git_mocks()
        with (
            git_text_mock,
            git_bytes_mock,
            mock.patch.dict(
                os.environ,
                {"GITHUB_TOKEN": "", "GITHUB_API_URL": "http://127.0.0.1:1"},
                clear=False,
            ),
        ):
            with self.assertRaisesRegex(
                verifier.VerificationError,
                "GITHUB_TOKEN",
            ):
                verifier.verify(self.release_args())

    def test_cross_origin_redirect_strips_authorization(self) -> None:
        received_authorization: list[str | None] = []

        class TargetHandler(BaseHTTPRequestHandler):
            def log_message(self, format: str, *args: object) -> None:
                del format, args

            def do_GET(self) -> None:
                received_authorization.append(
                    self.headers.get("Authorization")
                )
                body = b"redirected artifact"
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        target_server = ThreadingHTTPServer(("127.0.0.1", 0), TargetHandler)
        target_host, target_port = target_server.server_address
        target_url = f"http://{target_host}:{target_port}/artifact"

        class SourceHandler(BaseHTTPRequestHandler):
            def log_message(self, format: str, *args: object) -> None:
                del format, args

            def do_GET(self) -> None:
                if self.headers.get("Authorization") != "Bearer redirect-token":
                    self.send_response(401)
                    self.end_headers()
                    return
                self.send_response(302)
                self.send_header("Location", target_url)
                self.end_headers()

        source_server = ThreadingHTTPServer(("127.0.0.1", 0), SourceHandler)
        source_host, source_port = source_server.server_address
        source_thread = threading.Thread(
            target=source_server.serve_forever,
            daemon=True,
        )
        target_thread = threading.Thread(
            target=target_server.serve_forever,
            daemon=True,
        )
        target_thread.start()
        source_thread.start()
        try:
            client = verifier.GitHubClient(
                "redirect-token",
                f"http://{source_host}:{source_port}",
            )
            result = client._get(
                f"http://{source_host}:{source_port}/redirect",
                1024,
            )
        finally:
            source_server.shutdown()
            target_server.shutdown()
            source_server.server_close()
            target_server.server_close()
            source_thread.join(timeout=5)
            target_thread.join(timeout=5)

        self.assertEqual(result, b"redirected artifact")
        self.assertEqual(received_authorization, [None])

    def test_redirect_handler_preserves_same_origin_and_rejects_downgrade(
        self,
    ) -> None:
        handler = verifier.SafeRedirectHandler()
        request = urllib.request.Request(
            "https://example.test/source",
            headers={"Authorization": "Bearer token"},
        )
        redirected = handler.redirect_request(
            request,
            None,
            302,
            "Found",
            {},
            "https://example.test/target",
        )
        self.assertIsNotNone(redirected)
        assert redirected is not None
        self.assertEqual(redirected.get_header("Authorization"), "Bearer token")
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "HTTPS downgrade",
        ):
            handler.redirect_request(
                request,
                None,
                302,
                "Found",
                {},
                "http://example.test/target",
            )

    def test_release_rejects_tampered_zip_digest(self) -> None:
        self.artifacts["score-ci"].download += b"tampered"
        git_text_mock, git_bytes_mock = self.git_mocks()
        with MockGitHub(self.artifacts) as github:
            with (
                git_text_mock,
                git_bytes_mock,
                mock.patch.dict(
                    os.environ,
                    {
                        "GITHUB_TOKEN": "test-token",
                        "GITHUB_API_URL": github.api_url,
                    },
                    clear=False,
                ),
            ):
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    "downloaded ZIP digest",
                ):
                    verifier.verify(self.release_args())

    def test_release_rejects_untrusted_remote_metadata(self) -> None:
        scenarios = (
            ("fake-url", "archive_download_url"),
            ("zero-digest", "all-zero digest"),
            ("failed-run", "not completed successfully"),
        )
        for scenario, expected_error in scenarios:
            with self.subTest(scenario=scenario):
                _, artifacts = make_fixture()
                if scenario == "fake-url":
                    artifacts["score-ci"].fake_download_url = (
                        "https://attacker.invalid/forged.zip"
                    )
                elif scenario == "zero-digest":
                    artifacts["score-ci"].digest = "0" * 64
                else:
                    artifacts["score-ci"].run_conclusion = "failure"
                git_text_mock, git_bytes_mock = self.git_mocks()
                with MockGitHub(artifacts) as github:
                    with (
                        git_text_mock,
                        git_bytes_mock,
                        mock.patch.dict(
                            os.environ,
                            {
                                "GITHUB_TOKEN": "test-token",
                                "GITHUB_API_URL": github.api_url,
                            },
                            clear=False,
                        ),
                    ):
                        with self.assertRaisesRegex(
                            verifier.VerificationError,
                            expected_error,
                        ):
                            verifier.verify(self.release_args())

    def test_complete_four_artifact_classes_score_96(self) -> None:
        git_text_mock, git_bytes_mock = self.git_mocks()
        with MockGitHub(self.artifacts) as github:
            with (
                git_text_mock,
                git_bytes_mock,
                mock.patch.dict(
                    os.environ,
                    {
                        "GITHUB_TOKEN": "test-token",
                        "GITHUB_API_URL": github.api_url,
                    },
                    clear=False,
                ),
            ):
                result, passed = verifier.verify(self.release_args())

        self.assertTrue(passed)
        self.assertEqual(result["verdict"], "pass")
        self.assertEqual(result["score_verdict"], "pass")
        self.assertEqual(
            result["score"],
            {
                "baseline": 82,
                "target": 96,
                "awarded": 14,
                "total": 96,
                "complete": True,
                "target_met": True,
            },
        )
        self.assertEqual(
            github.requested_artifact_names,
            {
                "score-ci",
                "score-safety",
                "score-platform",
                "score-performance",
            },
        )
        expected_tracked_digest = hashlib.sha256(TRACKED_CONTENT).hexdigest()
        for condition in result["score_ledger"]:
            self.assertEqual(condition["status"], "passed")
            self.assertEqual(condition["awarded"], 1)
            self.assertEqual(
                condition["tracked_evidence"],
                [{"path": "Cargo.toml", "sha256": expected_tracked_digest}],
            )
            self.assertEqual(len(condition["required_attestations"]), 1)

    def test_current_manifest_scores_96_with_workflow_path_at_ref(self) -> None:
        current_manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        artifacts = make_artifacts_for_manifest(
            current_manifest,
            workflow_ref_suffix="@refs/heads/main",
        )
        args = argparse.Namespace(
            manifest=CURRENT_MANIFEST_PATH,
            comparison=None,
            mode="pr",
            release_only=True,
            github_repository=REPOSITORY,
            candidate_package=self.write_candidate_package_from_artifacts(
                current_manifest,
                artifacts,
            ),
            output=self.root / "current-result.json",
        )

        def current_git_bytes(*git_args: str) -> bytes:
            if (
                len(git_args) == 2
                and git_args[0] == "show"
                and git_args[1].startswith(f"{HEAD_SHA}:")
            ):
                return f"tracked:{git_args[1]}".encode()
            raise AssertionError(f"unexpected git invocation: {git_args}")

        with MockGitHub(artifacts) as github:
            with (
                mock.patch.object(verifier, "git_text", return_value=HEAD_SHA),
                mock.patch.object(
                    verifier,
                    "git_bytes",
                    side_effect=current_git_bytes,
                ),
                mock.patch.dict(
                    os.environ,
                    {
                        "GITHUB_TOKEN": "test-token",
                        "GITHUB_API_URL": github.api_url,
                    },
                    clear=False,
                ),
            ):
                result, passed = verifier.verify(args)

        self.assertTrue(passed)
        self.assertEqual(result["score"]["total"], 96)
        self.assertTrue(result["score"]["complete"])
        self.assertEqual(
            github.requested_artifact_names,
            {
                "cheetah-score-attestation-ci",
                "cheetah-score-attestation-safety",
                "cheetah-score-attestation-downstream",
                "cheetah-score-attestation-performance",
            },
        )
        downstream = next(
            condition
            for condition in result["score_ledger"]
            if condition["id"] == "engineering-downstream-and-platforms"
        )
        self.assertEqual(len(downstream["required_attestations"]), 2)
        traffic = next(
            attestation
            for attestation in downstream["required_attestations"]
            if attestation["claim"] == verifier.DOWNSTREAM_TRAFFIC_CLAIM
        )["downstream_traffic"]
        platforms = next(
            attestation
            for attestation in downstream["required_attestations"]
            if attestation["claim"] == verifier.CI_PLATFORMS_CLAIM
        )["ci_platforms"]
        self.assertEqual(
            traffic["crate"]["candidate_git_sha"],
            HEAD_SHA,
        )
        self.assertEqual(len(platforms["platforms"]), 3)
        self.assertEqual(
            {entry["runner_arch"] for entry in platforms["platforms"]},
            {"X64", "ARM64"},
        )
        for platform in platforms["platforms"]:
            self.assertEqual(
                set(platform["logs"]),
                {"all_features", "no_default_features"},
            )
        safety_condition = next(
            condition
            for condition in result["score_ledger"]
            if condition["id"] == "safety-utf8-and-unsafe"
        )
        safety = safety_condition["required_attestations"][0]["safety_stable"]
        self.assertEqual(len(safety["files"]), 4)
        self.assertEqual(
            {
                entry["max_total_time_seconds"]
                for entry in safety["files"]
                if entry["kind"] == "fuzz"
            },
            {3600},
        )
        self.assertEqual(traffic["matrix_counts"]["workspace"], 20)
        self.assertEqual(traffic["matrix_counts"]["standalone"], 2)
        self.assertEqual(len(traffic["rounds"]), 2)
        self.assertEqual(
            {
                artifact["role"]
                for artifact in traffic["input_artifacts"]
            },
            verifier.REQUIRED_DOWNSTREAM_INPUTS,
        )
        for condition in result["score_ledger"]:
            for attestation in condition["required_attestations"]:
                self.assertTrue(
                    attestation["workflow"].endswith("@refs/heads/main")
                )

    def test_ci_platform_payload_binds_descriptors_and_raw_logs(self) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        fields, raw_files = make_ci_platform_files(manifest)
        files = with_score_attestation(
            fields,
            raw_files,
            "engineering-downstream-and-platforms",
            verifier.CI_PLATFORMS_CLAIM,
        )
        result = self.validate_ci_platform_files(manifest, files)
        self.assertEqual(
            {entry["matrix_os"] for entry in result["platforms"]},
            {
                "macos-15-intel",
                "windows-latest",
                "ubuntu-24.04-arm",
            },
        )
        for entry in result["platforms"]:
            self.assertEqual(
                set(entry["logs"]),
                {"all_features", "no_default_features"},
            )

    def test_ci_platform_payload_fails_closed_on_missing_or_tampered_data(
        self,
    ) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )

        fields, raw_files = make_ci_platform_files(manifest)
        fields["platform_files"].pop()
        files = with_score_attestation(
            fields,
            raw_files,
            "engineering-downstream-and-platforms",
            verifier.CI_PLATFORMS_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "must contain exactly 3 platforms",
        ):
            self.validate_ci_platform_files(manifest, files)

        fields, raw_files = make_ci_platform_files(manifest)
        files = with_score_attestation(
            fields,
            raw_files,
            "engineering-downstream-and-platforms",
            verifier.CI_PLATFORMS_CLAIM,
        )
        tampered_name = (
            "platform-macos-15-intel-all-features.log"
        )
        files[tampered_name] += b"tampered\n"
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "SHA-256 does not match artifact content",
        ):
            self.validate_ci_platform_files(manifest, files)

    def test_ci_platform_payload_rejects_candidate_and_platform_mismatch(
        self,
    ) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        scenarios = (
            (
                "candidate",
                "candidate_git_sha",
                "3" * 40,
                "candidate_git_sha does not match HEAD",
            ),
            (
                "platform",
                "runner_arch",
                "ARM64",
                "runner_arch does not match policy",
            ),
        )
        for name, field, value, expected_error in scenarios:
            with self.subTest(name=name):
                fields, raw_files = make_ci_platform_files(manifest)
                descriptor_name = "platform-macos-15-intel.json"
                descriptor = json.loads(raw_files[descriptor_name])
                descriptor[field] = value
                descriptor_raw = json_bytes(descriptor)
                raw_files[descriptor_name] = descriptor_raw
                fields["platform_files"][0]["descriptor"][
                    "sha256"
                ] = hashlib.sha256(descriptor_raw).hexdigest()
                files = with_score_attestation(
                    fields,
                    raw_files,
                    "engineering-downstream-and-platforms",
                    verifier.CI_PLATFORMS_CLAIM,
                )
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.validate_ci_platform_files(manifest, files)

    def test_safety_payload_binds_logs_markers_and_release_duration(self) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        fields, raw_files = make_safety_files(manifest)
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        result = self.validate_safety_files(manifest, files)
        self.assertEqual(len(result["files"]), 4)
        self.assertEqual(
            {
                entry["max_total_time_seconds"]
                for entry in result["files"]
                if entry["kind"] == "fuzz"
            },
            {3600},
        )

    def test_safety_payload_fails_closed_on_missing_tamper_and_candidate(
        self,
    ) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )

        fields, raw_files = make_safety_files(manifest)
        raw_files.pop("miri-stable.log")
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "exactly one root miri-stable.log",
        ):
            self.validate_safety_files(manifest, files)

        fields, raw_files = make_safety_files(manifest)
        raw_files["asan-stable.log"] += b"tampered\n"
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "SHA-256 does not match artifact content",
        ):
            self.validate_safety_files(manifest, files)

        fields, raw_files = make_safety_files(manifest)
        filename = "miri-stable.log"
        raw_files[filename] = raw_files[filename].replace(
            HEAD_SHA.encode(),
            ("3" * 40).encode(),
        )
        miri = next(
            entry for entry in fields["safety_files"] if entry["id"] == "miri"
        )
        miri["sha256"] = hashlib.sha256(raw_files[filename]).hexdigest()
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "missing candidate, target, or passing markers",
        ):
            self.validate_safety_files(manifest, files)

    def test_safety_payload_rejects_missing_test_marker_and_short_fuzz(
        self,
    ) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        policy = manifest["safety_stable_policy"]

        fields, raw_files = make_safety_files(manifest)
        filename = "asan-stable.log"
        raw_files[filename] = raw_files[filename].replace(
            b"test result: ok.\n",
            b"",
        )
        asan = next(
            entry for entry in fields["safety_files"] if entry["id"] == "asan"
        )
        asan["sha256"] = hashlib.sha256(raw_files[filename]).hexdigest()
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "has no passing test marker",
        ):
            self.validate_safety_files(manifest, files)

        fields, raw_files = make_safety_files(manifest)
        fuzz = next(
            entry
            for entry in fields["safety_files"]
            if entry["kind"] == "fuzz"
        )
        filename = fuzz["filename"]
        short_duration = policy["minimum_fuzz_seconds"] - 1
        raw_files[filename] = raw_files[filename].replace(
            f"max_total_time={policy['minimum_fuzz_seconds']}".encode(),
            f"max_total_time={short_duration}".encode(),
        )
        fuzz["max_total_time_seconds"] = short_duration
        fuzz["sha256"] = hashlib.sha256(raw_files[filename]).hexdigest()
        files = with_score_attestation(
            fields,
            raw_files,
            "safety-utf8-and-unsafe",
            verifier.SAFETY_STABLE_CLAIM,
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "duration is below 3600 seconds",
        ):
            self.validate_safety_files(manifest, files)

    def test_platform_and_safety_policies_are_versioned_and_locked(self) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        weakened_platform = json.loads(json.dumps(manifest))
        weakened_platform["ci_platform_policy"]["required_workflow_needs"].remove(
            "layout-32"
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "must match the locked",
        ):
            verifier.validate_ci_platform_policy(weakened_platform)

        weakened_safety = json.loads(json.dumps(manifest))
        weakened_safety["safety_stable_policy"]["minimum_fuzz_seconds"] = 3599
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "must match the locked",
        ):
            verifier.validate_safety_stable_policy(weakened_safety)

    def test_release_manifest_is_byte_locked_before_remote_evidence(self) -> None:
        digest = self.real_validate_release_manifest(CURRENT_MANIFEST_PATH)
        self.assertEqual(digest, verifier.CANONICAL_RELEASE_MANIFEST_SHA256)

        weakened = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        weakened["score"]["baseline"] = 93
        weakened["score"]["conditions"] = [
            condition
            for condition in weakened["score"]["conditions"]
            if condition["assessment"] == "comparison"
        ]
        path = self.root / "weakened-manifest.json"
        path.write_text(json.dumps(weakened), encoding="utf-8")
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "canonical reviewed contract",
        ):
            self.real_validate_release_manifest(path)

    def test_downstream_identity_and_package_are_exact_candidate_bound(self) -> None:
        scenarios = (
            (
                "candidate",
                ("candidate_git_sha",),
                "3" * 40,
                "candidate_git_sha does not match HEAD",
            ),
            (
                "crate-candidate",
                ("crate", "candidate_git_sha"),
                "3" * 40,
                "crate.candidate_git_sha does not match HEAD",
            ),
            (
                "crate-version",
                ("crate", "version"),
                "3.0.0-forged",
                "crate.version does not match policy",
            ),
            (
                "package-digest",
                ("crate", "package_sha256"),
                "b" * 64,
                "package SHA-256 does not match artifact content",
            ),
            (
                "downstream-source",
                ("downstream", "source_git_sha"),
                "3" * 40,
                "downstream.source_git_sha does not match policy",
            ),
            (
                "workload",
                ("workload_id",),
                "synthetic-smoke",
                "workload_id does not match policy",
            ),
        )
        for name, path, value, expected_error in scenarios:
            with self.subTest(name=name):
                manifest = json.loads(
                    CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
                )
                files = make_downstream_files(manifest)
                evidence = json.loads(
                    files[verifier.DOWNSTREAM_TRAFFIC_FILENAME]
                )
                target: dict[str, Any] = evidence
                for part in path[:-1]:
                    target = target[part]
                target[path[-1]] = value
                files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.validate_downstream_files(manifest, files)

        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        files = make_downstream_files(manifest)
        policy_crate = manifest["downstream_traffic_policy"]["crate"]
        forged_package = make_crate_package(
            policy_crate["name"],
            policy_crate["version"],
            "3" * 40,
        )
        files[policy_crate["package_filename"]] = forged_package
        evidence = json.loads(files[verifier.DOWNSTREAM_TRAFFIC_FILENAME])
        evidence["crate"]["package_sha256"] = hashlib.sha256(
            forged_package
        ).hexdigest()
        files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "Cargo package VCS SHA does not match exact candidate",
        ):
            self.validate_downstream_files(manifest, files)

        files = make_downstream_files(manifest)
        different_valid_candidate = make_crate_package(
            policy_crate["name"],
            policy_crate["version"],
            HEAD_SHA,
            readme=b"same VCS identity, different validated bytes\n",
        )
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "independently validated candidate package",
        ):
            self.validate_downstream_files(
                manifest,
                files,
                candidate_package=different_valid_candidate,
            )

    def test_downstream_provenance_matrix_and_protocol_fail_closed(self) -> None:
        scenarios = (
            ("missing-lock", "input", "cargo_lock", "input_artifacts must contain"),
            ("tampered-config", "digest", "build_config", "SHA-256 does not match"),
            (
                "missing-feature",
                "feature",
                "",
                "missing required crate feature profiles",
            ),
            (
                "failed-package",
                "package-status",
                "",
                "downstream package .* did not pass",
            ),
            (
                "short-package-matrix",
                "package-count",
                "",
                "workspace package count is below policy minimum",
            ),
            (
                "short-warmup",
                "measurement",
                "warmup_seconds",
                "warmup_seconds is below policy minimum",
            ),
            (
                "short-duration",
                "measurement",
                "duration_seconds",
                "duration_seconds is below policy minimum",
            ),
            (
                "few-samples",
                "samples",
                "",
                "sample_count is below policy minimum",
            ),
            (
                "one-round",
                "rounds",
                "",
                "minimum independent measurement rounds",
            ),
        )
        for name, action, field, expected_error in scenarios:
            with self.subTest(name=name):
                manifest = json.loads(
                    CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
                )
                files = make_downstream_files(manifest)
                evidence = json.loads(
                    files[verifier.DOWNSTREAM_TRAFFIC_FILENAME]
                )
                if action == "input":
                    evidence["input_artifacts"].pop(field)
                elif action == "digest":
                    evidence["input_artifacts"][field]["sha256"] = "b" * 64
                elif action == "feature":
                    evidence["crate_feature_matrix"].pop()
                elif action == "package-status":
                    evidence["package_matrix"]["workspace"][0]["status"] = "failed"
                elif action == "package-count":
                    evidence["package_matrix"]["workspace"].pop()
                elif action == "measurement":
                    evidence["measurement"][field] -= 1
                elif action == "samples":
                    evidence["measurement"]["rounds"][0]["sample_count"] -= 1
                else:
                    evidence["measurement"]["rounds"] = evidence["measurement"][
                        "rounds"
                    ][:1]
                files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.validate_downstream_files(manifest, files)

    def test_downstream_all_metric_thresholds_fail_closed(self) -> None:
        for metric in sorted(verifier.DOWNSTREAM_METRICS):
            with self.subTest(metric=metric):
                manifest = json.loads(
                    CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
                )
                files = make_downstream_files(manifest)
                evidence = json.loads(
                    files[verifier.DOWNSTREAM_TRAFFIC_FILENAME]
                )
                round_evidence = evidence["measurement"]["rounds"][0]
                round_filename = round_evidence["raw_artifact"]["filename"]
                raw_round = json.loads(files[round_filename])
                candidate = raw_round["candidate_samples"]
                if metric == "throughput_ops_per_second":
                    for sample in candidate:
                        sample["elapsed_ns"] = 1_041_667
                elif metric == "latency_p50_ms":
                    for sample in candidate[:50]:
                        sample["latency_ns"] = 10_400_000
                elif metric == "latency_p95_ms":
                    for sample in candidate[50:95]:
                        sample["latency_ns"] = 20_800_000
                elif metric == "latency_p99_ms":
                    for sample in candidate[95:]:
                        sample["latency_ns"] = 31_200_000
                elif metric == "cpu_time_per_operation_ns":
                    for sample in candidate:
                        sample["cpu_time_ns"] = 104
                elif metric == "rss_peak_bytes":
                    for sample in candidate:
                        sample["rss_peak_bytes"] = 1040
                else:
                    for index, sample in enumerate(candidate):
                        sample["allocations"] = 11 if index < 40 else 10
                raw = json_bytes(raw_round)
                files[round_filename] = raw
                round_evidence["raw_artifact"]["sha256"] = hashlib.sha256(
                    raw
                ).hexdigest()
                round_evidence["metrics"]["candidate"] = fixture_round_metrics(
                    candidate
                )
                files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    metric,
                ):
                    self.validate_downstream_files(manifest, files)

    def test_downstream_raw_round_artifacts_are_digest_bound_and_distinct(self) -> None:
        manifest = json.loads(
            CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
        )
        files = make_downstream_files(manifest)
        evidence = json.loads(files[verifier.DOWNSTREAM_TRAFFIC_FILENAME])
        evidence["measurement"]["rounds"][0]["raw_artifact"]["sha256"] = "b" * 64
        files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "raw_artifact SHA-256 does not match artifact content",
        ):
            self.validate_downstream_files(manifest, files)

        files = make_downstream_files(manifest)
        evidence = json.loads(files[verifier.DOWNSTREAM_TRAFFIC_FILENAME])
        round_one = evidence["measurement"]["rounds"][0]["raw_artifact"]
        round_two = evidence["measurement"]["rounds"][1]["raw_artifact"]
        files[round_two["filename"]] = files[round_one["filename"]]
        round_two["sha256"] = round_one["sha256"]
        files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "reuse identical raw traffic evidence",
        ):
            self.validate_downstream_files(manifest, files)

        files = make_downstream_files(manifest)
        evidence = json.loads(files[verifier.DOWNSTREAM_TRAFFIC_FILENAME])
        round_one = evidence["measurement"]["rounds"][0]["raw_artifact"]
        round_two = evidence["measurement"]["rounds"][1]["raw_artifact"]
        raw_one = json.loads(files[round_one["filename"]])
        raw_two = json.loads(files[round_two["filename"]])
        raw_two["baseline_samples"] = raw_one["baseline_samples"]
        raw_two["candidate_samples"] = raw_one["candidate_samples"]
        files[round_two["filename"]] = (
            json.dumps(raw_two, indent=2, ensure_ascii=False).encode() + b"\n"
        )
        round_two["sha256"] = hashlib.sha256(
            files[round_two["filename"]]
        ).hexdigest()
        files[verifier.DOWNSTREAM_TRAFFIC_FILENAME] = json_bytes(evidence)
        with self.assertRaisesRegex(
            verifier.VerificationError,
            "semantically identical samples",
        ):
            self.validate_downstream_files(manifest, files)

    def test_downstream_policy_cannot_weaken_release_boundaries(self) -> None:
        scenarios = (
            ("workspace", ("package_matrix", "minimum_workspace_packages"), 19),
            ("standalone", ("package_matrix", "minimum_standalone_packages"), 1),
            ("rounds", ("measurement", "minimum_rounds"), 1),
            ("warmup", ("measurement", "minimum_warmup_seconds"), 1),
            ("duration", ("measurement", "minimum_duration_seconds"), 1),
            ("samples", ("measurement", "minimum_samples_per_round"), 1),
            (
                "feature-profiles",
                ("package_matrix", "required_crate_feature_profiles"),
                ["default"],
            ),
            ("workload", ("workload_id",), "weakened-workload"),
            (
                "source",
                ("downstream", "source_git_sha"),
                "3" * 40,
            ),
            (
                "throughput",
                (
                    "measurement",
                    "metric_thresholds",
                    "throughput_ops_per_second",
                    "minimum_candidate_to_baseline_ratio",
                ),
                0.96,
            ),
            (
                "p99",
                (
                    "measurement",
                    "metric_thresholds",
                    "latency_p99_ms",
                    "maximum_candidate_to_baseline_ratio",
                ),
                1.04,
            ),
        )
        for name, path, value in scenarios:
            with self.subTest(name=name):
                manifest = json.loads(
                    CURRENT_MANIFEST_PATH.read_text(encoding="utf-8-sig")
                )
                target = manifest["downstream_traffic_policy"]
                for part in path[:-1]:
                    target = target[part]
                target[path[-1]] = value
                with self.assertRaises(verifier.VerificationError):
                    verifier.validate_downstream_policy(manifest)

    def test_performance_attestation_cannot_self_report_failed_comparison(self) -> None:
        performance = self.artifacts["score-performance"]
        failed_comparison = {
            "schema_version": 2,
            "manifest_id": MANIFEST_ID,
            "mode": "final",
            "head_git_sha": HEAD_SHA,
            "base_git_sha": BASE_SHA,
            "metadata_compatible": True,
            "performance_verdict": "fail",
            "score_ledger": [
                {"id": condition_id, "status": "passed"}
                for condition_id in COMPARISON_IDS
            ],
        }
        performance.archive = replace_zip_json(
            performance.archive,
            "comparison-r2.json",
            failed_comparison,
        )
        performance.download = performance.archive
        performance.digest = hashlib.sha256(performance.archive).hexdigest()
        git_text_mock, git_bytes_mock = self.git_mocks()
        with MockGitHub(self.artifacts) as github:
            with (
                git_text_mock,
                git_bytes_mock,
                mock.patch.dict(
                    os.environ,
                    {
                        "GITHUB_TOKEN": "test-token",
                        "GITHUB_API_URL": github.api_url,
                    },
                    clear=False,
                ),
            ):
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    "comparison-r2.json performance_verdict is not pass",
                ):
                    verifier.verify(self.release_args())

    def test_final_comparison_passes_performance_with_incomplete_score(self) -> None:
        result, passed = self.verify_comparison_value(
            make_comparison_result("final"),
            "final",
            "comparison-final.json",
        )

        self.assertTrue(passed)
        self.assertEqual(result["verdict"], "pass")
        self.assertEqual(result["gate_scope"], "final-performance")
        self.assertEqual(result["score_verdict"], "incomplete")
        self.assertEqual(result["score"]["total"], 85)
        self.assertFalse(result["score"]["complete"])
        self.assertFalse(result["score"]["target_met"])

    def test_pr_comparison_passes_with_final_sentinel(self) -> None:
        result, passed = self.verify_comparison_value(
            make_comparison_result("pr"),
            "pr",
            "comparison-pr.json",
        )

        self.assertTrue(passed)
        self.assertEqual(result["gate_scope"], "pr-performance")
        self.assertEqual(result["score_verdict"], "incomplete")
        self.assertEqual(result["score"]["total"], 84)
        statuses = {
            entry["id"]: entry["status"]
            for entry in result["score_ledger"]
            if entry["assessment"] == "comparison"
        }
        self.assertEqual(
            statuses,
            {
                "performance-zero-allocation-clone": "passed",
                "performance-final-geomean": "final-comparison-required",
                "performance-fixed-runner-budgets": "passed",
            },
        )

    def test_comparison_requires_schema_two_and_compatible_metadata(self) -> None:
        scenarios = (
            ("schema", "schema_version", 1, "schema_version must be 2"),
            (
                "metadata",
                "metadata_compatible",
                False,
                "metadata_compatible must be true",
            ),
        )
        for name, field, value, expected_error in scenarios:
            with self.subTest(name=name):
                comparison = make_comparison_result("final")
                comparison[field] = value
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.verify_comparison_value(
                        comparison,
                        "final",
                        f"comparison-{name}.json",
                    )

    def test_comparison_validates_mode_specific_base_git_sha(self) -> None:
        scenarios = (
            (
                "final-nonfrozen",
                "final",
                "3" * 40,
                "base_git_sha is not frozen baseline",
            ),
            (
                "pr-invalid-length",
                "pr",
                "3" * 41,
                "base_git_sha is not a valid Git object id",
            ),
            (
                "pr-all-zero",
                "pr",
                "0" * 40,
                "base_git_sha cannot be the all-zero Git object id",
            ),
        )
        for name, mode, base_git_sha, expected_error in scenarios:
            with self.subTest(name=name):
                comparison = make_comparison_result(
                    mode,
                    base_git_sha=base_git_sha,
                )
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.verify_comparison_value(
                        comparison,
                        mode,
                        f"comparison-{name}.json",
                    )

    def test_pass_verdict_requires_mode_specific_comparison_states(self) -> None:
        scenarios = (
            (
                "final-failed-budget",
                "final",
                "performance-fixed-runner-budgets",
                "failed",
                "performance-fixed-runner-budgets status passed",
            ),
            (
                "pr-failed-clone",
                "pr",
                "performance-zero-allocation-clone",
                "failed",
                "performance-zero-allocation-clone status passed",
            ),
            (
                "pr-forged-final-geomean",
                "pr",
                "performance-final-geomean",
                "passed",
                (
                    "performance-final-geomean status "
                    "final-comparison-required"
                ),
            ),
        )
        for name, mode, condition_id, status, expected_error in scenarios:
            with self.subTest(name=name):
                comparison = make_comparison_result(mode)
                for entry in comparison["score_ledger"]:
                    if entry["id"] == condition_id:
                        entry["status"] = status
                        break
                with self.assertRaisesRegex(
                    verifier.VerificationError,
                    expected_error,
                ):
                    self.verify_comparison_value(
                        comparison,
                        mode,
                        f"comparison-{name}.json",
                    )

    def test_cli_rejects_self_reported_evidence_argument(self) -> None:
        stderr = io.StringIO()
        argv = [
            "verify-score.py",
            "--manifest",
            str(self.manifest_path),
            "--comparison",
            str(self.root / "comparison.json"),
            "--output",
            str(self.root / "result.json"),
            "--evidence",
            str(self.root / "forged.json"),
        ]
        with (
            mock.patch.object(sys, "argv", argv),
            contextlib.redirect_stderr(stderr),
            self.assertRaises(SystemExit) as raised,
        ):
            verifier.parse_args()
        self.assertEqual(raised.exception.code, 2)
        self.assertIn("unrecognized arguments: --evidence", stderr.getvalue())

    def test_cli_requires_candidate_package_for_release(self) -> None:
        stderr = io.StringIO()
        argv = [
            "verify-score.py",
            "--manifest",
            str(self.manifest_path),
            "--release-only",
            "--github-repository",
            REPOSITORY,
            "--output",
            str(self.root / "result.json"),
        ]
        with (
            mock.patch.object(sys, "argv", argv),
            contextlib.redirect_stderr(stderr),
            self.assertRaises(SystemExit) as raised,
        ):
            verifier.parse_args()
        self.assertEqual(raised.exception.code, 2)
        self.assertIn("--candidate-package is required", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
