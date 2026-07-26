#!/usr/bin/env python3
"""Fail-closed verifier for the versioned score gate.

Comparison verification is intentionally limited to the performance scope.
Release scoring additionally requires GitHub-hosted, immutable attestations for
the exact checkout being verified.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import os
import re
import subprocess
import sys
import tarfile
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


GIT_OBJECT = re.compile(r"^(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})$")
GITHUB_REPOSITORY = re.compile(
    r"^(?P<owner>[A-Za-z0-9_.-]+)/(?P<repository>[A-Za-z0-9_.-]+)$"
)
GITHUB_WORKFLOW_REF = re.compile(
    r"^refs/(?:heads|tags|pull)/[A-Za-z0-9._/-]+$"
)
IMMUTABLE_DIGEST = re.compile(r"^sha256:(?P<value>[0-9a-fA-F]{64})$")
PERFORMANCE_FINAL_CLAIM = "performance-final"
PERFORMANCE_FINAL_GEOMEAN = "performance-final-geomean"
CI_PLATFORMS_CLAIM = "ci-platforms"
SAFETY_STABLE_CLAIM = "safety-stable"
DOWNSTREAM_TRAFFIC_CLAIM = "downstream-traffic"
DOWNSTREAM_TRAFFIC_FILENAME = "downstream-traffic.json"
DOWNSTREAM_ROUND_SCHEMA_VERSION = 2
# These digests are updated only when the reviewed release gate contract changes.
# The manifest digest protects the complete score ledger and every attestation
# binding; the policy digest also makes direct policy validation fail closed.
CANONICAL_RELEASE_MANIFEST_SHA256 = (
    "3e46e9b582316a2754c611982250ddafab4c081961415f77e1152df492b7a52e"
)
LOCKED_DOWNSTREAM_TRAFFIC_POLICY_SHA256 = (
    "3514aaf52878e0fce506ffd7023d39bb8fbd27127a203e285aebd925c4eb60fa"
)
LOCKED_CI_PLATFORM_POLICY = {
    "schema_version": 1,
    "policy_id": "hosted-platforms-and-i686-layout-v2",
    "workflow": ".github/workflows/ci.yaml",
    "score_job": "score-attestation",
    "required_workflow_needs": [
        "deterministic",
        "msrv",
        "platforms",
        "layout-32",
    ],
    "descriptor_schema_version": 1,
    "required_checks": [
        {
            "id": "all_features",
            "log_filename": "all-features.log",
            "passing_marker": "test result: ok.",
        },
        {
            "id": "no_default_features",
            "log_filename": "no-default.log",
            "passing_marker": "Finished",
        },
    ],
    "platforms": [
        {
            "matrix_os": "macos-15-intel",
            "runner_os": "macOS",
            "runner_arch": "X64",
            "rustc_host": "x86_64-apple-darwin",
        },
        {
            "matrix_os": "windows-latest",
            "runner_os": "Windows",
            "runner_arch": "X64",
            "rustc_host": "x86_64-pc-windows-msvc",
        },
        {
            "matrix_os": "ubuntu-24.04-arm",
            "runner_os": "Linux",
            "runner_arch": "ARM64",
            "rustc_host": "aarch64-unknown-linux-gnu",
        },
    ],
    "layout_gate": {
        "job": "layout-32",
        "target": "i686-pc-windows-msvc",
        "test": "layout_snapshot",
    },
}
LOCKED_SAFETY_STABLE_POLICY = {
    "schema_version": 1,
    "policy_id": "release-safety-stable-v1",
    "workflow": ".github/workflows/safety.yml",
    "score_job": "score-attestation",
    "required_workflow_needs": ["miri", "asan", "fuzz"],
    "passing_marker": "safety_check_status=passed",
    "minimum_fuzz_seconds": 3600,
    "required_logs": [
        {
            "id": "miri",
            "filename": "miri-stable.log",
            "kind": "test",
            "target": "x86_64-unknown-linux-gnu",
            "test_marker": "test result: ok.",
        },
        {
            "id": "asan",
            "filename": "asan-stable.log",
            "kind": "test",
            "target": "x86_64-unknown-linux-gnu",
            "test_marker": "test result: ok.",
        },
        {
            "id": "fuzz-cheetah-string-transitions",
            "filename": "fuzz-cheetah_string_transitions.log",
            "kind": "fuzz",
            "target": "cheetah_string_transitions",
        },
        {
            "id": "fuzz-split-differential",
            "filename": "fuzz-split_differential.log",
            "kind": "fuzz",
            "target": "split_differential",
        },
    ],
}
REQUIRED_DOWNSTREAM_INPUTS = frozenset(
    {
        "cargo_lock",
        "dependency_graph",
        "build_config",
        "dataset_summary",
        "package_matrix",
    }
)
DOWNSTREAM_METRICS = frozenset(
    {
        "throughput_ops_per_second",
        "latency_p50_ms",
        "latency_p95_ms",
        "latency_p99_ms",
        "cpu_time_per_operation_ns",
        "rss_peak_bytes",
        "allocations_per_operation",
    }
)
DOWNSTREAM_HIGHER_IS_BETTER = frozenset({"throughput_ops_per_second"})
SHA256_HEX = re.compile(r"^[0-9a-f]{64}$")
EXPECTED_COMPARISON_CONDITIONS = frozenset(
    {
        "performance-zero-allocation-clone",
        PERFORMANCE_FINAL_GEOMEAN,
        "performance-fixed-runner-budgets",
    }
)
MAX_ARCHIVE_BYTES = 256 * 1024 * 1024
MAX_JSON_BYTES = 16 * 1024 * 1024


class VerificationError(RuntimeError):
    """An input or remote attestation failed closed."""


def normalized_http_origin(url: str) -> tuple[str, str, int]:
    parsed = urllib.parse.urlsplit(url)
    scheme = parsed.scheme.casefold()
    hostname = (parsed.hostname or "").casefold()
    if scheme not in {"http", "https"} or not hostname:
        raise VerificationError(f"redirect target is not an HTTP(S) URL: {url}")
    try:
        port = parsed.port
    except ValueError as exc:
        raise VerificationError(f"redirect target has an invalid port: {url}") from exc
    if port is None:
        port = 443 if scheme == "https" else 80
    return scheme, hostname, port


class SafeRedirectHandler(urllib.request.HTTPRedirectHandler):
    """Preserve credentials only for same-origin redirects."""

    def redirect_request(
        self,
        request: urllib.request.Request,
        file_pointer: Any,
        code: int,
        message: str,
        headers: Any,
        new_url: str,
    ) -> urllib.request.Request | None:
        source_origin = normalized_http_origin(request.full_url)
        target_origin = normalized_http_origin(new_url)
        if source_origin[0] == "https" and target_origin[0] != "https":
            raise VerificationError("refusing HTTPS downgrade redirect")
        redirected = super().redirect_request(
            request,
            file_pointer,
            code,
            message,
            headers,
            new_url,
        )
        if redirected is not None and source_origin != target_origin:
            for header in ("Authorization", "Proxy-Authorization", "Cookie"):
                redirected.remove_header(header)
        return redirected


@dataclass(frozen=True)
class VerifiedArtifact:
    name: str
    artifact_id: int
    digest: str
    run_id: int
    run_path: str
    run_event: str
    files: dict[str, bytes]
    claims: frozenset[tuple[str, str]]


class GitHubClient:
    """Small authenticated GitHub REST client used by release verification."""

    def __init__(self, token: str, api_url: str) -> None:
        parsed = urllib.parse.urlsplit(api_url)
        if (
            parsed.scheme not in {"http", "https"}
            or not parsed.netloc
            or parsed.username is not None
            or parsed.password is not None
            or parsed.query
            or parsed.fragment
        ):
            raise VerificationError("GITHUB_API_URL must be an HTTP(S) API base URL")
        if parsed.scheme == "http" and parsed.hostname not in {
            "127.0.0.1",
            "::1",
            "localhost",
        }:
            raise VerificationError(
                "plain HTTP GITHUB_API_URL is allowed only for a local mock server"
            )
        self.api_url = api_url.rstrip("/")
        self.headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "User-Agent": "cheetah-string-score-verifier",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        self.opener = urllib.request.build_opener(SafeRedirectHandler())

    def url(self, path: str, query: dict[str, Any] | None = None) -> str:
        if not path.startswith("/"):
            raise VerificationError(f"internal GitHub API path is not absolute: {path}")
        url = self.api_url + path
        if query:
            url += "?" + urllib.parse.urlencode(query)
        return url

    def get_json(
        self,
        path: str,
        query: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        url = self.url(path, query)
        raw = self._get(url, MAX_JSON_BYTES)
        try:
            value = json.loads(raw.decode("utf-8-sig"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise VerificationError(f"GitHub API returned invalid JSON for {url}: {exc}") from exc
        if not isinstance(value, dict):
            raise VerificationError(f"GitHub API did not return an object for {url}")
        return value

    def get_bytes(self, path: str) -> bytes:
        return self._get(self.url(path), MAX_ARCHIVE_BYTES)

    def _get(self, url: str, limit: int) -> bytes:
        request = urllib.request.Request(url, headers=self.headers, method="GET")
        try:
            with self.opener.open(request, timeout=30) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None:
                    try:
                        declared_length = int(content_length)
                    except ValueError as exc:
                        raise VerificationError(
                            f"GitHub API returned invalid Content-Length for {url}"
                        ) from exc
                    if declared_length < 0 or declared_length > limit:
                        raise VerificationError(
                            f"GitHub API response exceeds {limit} bytes for {url}"
                        )
                value = response.read(limit + 1)
        except VerificationError:
            raise
        except urllib.error.HTTPError as exc:
            raise VerificationError(
                f"GitHub API request failed ({exc.code}) for {url}"
            ) from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise VerificationError(f"GitHub API request failed for {url}: {exc}") from exc
        if len(value) > limit:
            raise VerificationError(f"GitHub API response exceeds {limit} bytes for {url}")
        return value


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError) as exc:
        raise VerificationError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise VerificationError(f"expected a JSON object: {path}")
    return value


def read_file_limited(path: Path, limit: int, label: str) -> bytes:
    try:
        if not path.is_file():
            raise VerificationError(f"{label} is not a regular file: {path}")
        size = path.stat().st_size
        if size < 0 or size > limit:
            raise VerificationError(f"{label} exceeds {limit} bytes: {path}")
        value = path.read_bytes()
    except VerificationError:
        raise
    except OSError as exc:
        raise VerificationError(f"cannot read {label} {path}: {exc}") from exc
    if len(value) > limit:
        raise VerificationError(f"{label} exceeds {limit} bytes: {path}")
    return value


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def validate_release_manifest(path: Path) -> str:
    digest = sha256_bytes(read_file_limited(path, MAX_JSON_BYTES, "release manifest"))
    if digest != CANONICAL_RELEASE_MANIFEST_SHA256:
        raise VerificationError(
            "release manifest SHA-256 does not match the canonical reviewed contract"
        )
    return digest


def load_json_bytes(value: bytes, label: str) -> dict[str, Any]:
    if len(value) > MAX_JSON_BYTES:
        raise VerificationError(f"{label} exceeds {MAX_JSON_BYTES} bytes")
    try:
        result = json.loads(value.decode("utf-8-sig"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise VerificationError(f"{label} is not valid JSON: {exc}") from exc
    if not isinstance(result, dict):
        raise VerificationError(f"{label} must contain a JSON object")
    return result


def git_bytes(*args: str) -> bytes:
    try:
        return subprocess.check_output(["git", *args], stderr=subprocess.STDOUT)
    except (OSError, subprocess.CalledProcessError) as exc:
        if isinstance(exc, subprocess.CalledProcessError):
            detail = exc.output.decode("utf-8", errors="replace").strip()
        else:
            detail = str(exc)
        raise VerificationError(f"git {' '.join(args)} failed: {detail}") from exc


def git_text(*args: str) -> str:
    return git_bytes(*args).decode("utf-8").strip()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require_unique(
    items: list[dict[str, Any]],
    key: str,
    label: str,
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for item in items:
        if not isinstance(item, dict):
            raise VerificationError(f"{label} entries must be objects")
        value = item.get(key)
        if not isinstance(value, str) or not value:
            raise VerificationError(f"{label} entry has no non-empty {key}")
        if value in result:
            raise VerificationError(f"duplicate {label} {key}: {value}")
        result[value] = item
    return result


def manifest_conditions(
    manifest: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], int, int]:
    manifest_id = manifest.get("manifest_id")
    if not isinstance(manifest_id, str) or not manifest_id:
        raise VerificationError("manifest_id must be a non-empty string")
    score = manifest.get("score")
    if not isinstance(score, dict):
        raise VerificationError("manifest score object is missing")
    raw_conditions = score.get("conditions")
    if not isinstance(raw_conditions, list):
        raise VerificationError("manifest score.conditions must be an array")
    conditions = require_unique(raw_conditions, "id", "manifest condition")
    baseline = score.get("baseline")
    target = score.get("target")
    if (
        not isinstance(baseline, int)
        or isinstance(baseline, bool)
        or not isinstance(target, int)
        or isinstance(target, bool)
    ):
        raise VerificationError("manifest baseline and target must be integers")

    maximum = baseline
    for condition in conditions.values():
        condition_id = condition["id"]
        points = condition.get("points")
        if not isinstance(points, int) or isinstance(points, bool) or points <= 0:
            raise VerificationError(f"condition {condition_id} has invalid points")
        if condition.get("assessment") not in {"comparison", "release-evidence"}:
            raise VerificationError(f"condition {condition_id} has invalid assessment")
        dimension = condition.get("dimension")
        if not isinstance(dimension, str) or not dimension:
            raise VerificationError(f"condition {condition_id} has invalid dimension")
        required_kinds = condition.get("required_evidence_kinds")
        if required_kinds is not None and (
            not isinstance(required_kinds, list)
            or not required_kinds
            or any(
                kind not in {"tracked-file", "verified-ci-attestation"}
                for kind in required_kinds
            )
            or len(set(required_kinds)) != len(required_kinds)
        ):
            raise VerificationError(
                f"condition {condition_id} has invalid required_evidence_kinds"
            )
        maximum += points
    if maximum < target:
        raise VerificationError(f"manifest maximum score {maximum} is below target {target}")
    return conditions, baseline, target


def parse_repository(value: str | None) -> tuple[str, str, str]:
    if not isinstance(value, str):
        raise VerificationError("--github-repository is required for --release-only")
    match = GITHUB_REPOSITORY.fullmatch(value.strip())
    if match is None:
        raise VerificationError(
            "--github-repository must use the exact OWNER/REPOSITORY form"
        )
    owner = match.group("owner")
    repository = match.group("repository")
    return owner, repository, f"{owner}/{repository}"


def positive_integer(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise VerificationError(f"{label} must be a positive integer")
    return value


def nonnegative_integer(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise VerificationError(f"{label} must be a non-negative integer")
    return value


def bounded_positive_integer(value: Any, label: str) -> int:
    result = positive_integer(value, label)
    if result > (1 << 63) - 1:
        raise VerificationError(f"{label} exceeds signed 64-bit range")
    return result


def bounded_nonnegative_integer(value: Any, label: str) -> int:
    result = nonnegative_integer(value, label)
    if result > (1 << 63) - 1:
        raise VerificationError(f"{label} exceeds signed 64-bit range")
    return result


def immutable_digest(value: Any, label: str) -> str:
    if not isinstance(value, str):
        raise VerificationError(f"{label} has no immutable SHA-256 digest")
    match = IMMUTABLE_DIGEST.fullmatch(value)
    if match is None:
        raise VerificationError(f"{label} has no immutable SHA-256 digest")
    digest = match.group("value").lower()
    if digest == "0" * 64:
        raise VerificationError(f"{label} uses the forbidden all-zero digest")
    return digest


def normalize_workflow_path(value: Any, condition_id: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise VerificationError(
            f"condition {condition_id} attestation workflow must be non-empty"
        )
    workflow = value.strip()
    if "/" not in workflow:
        workflow = f".github/workflows/{workflow}"
    if (
        not workflow.startswith(".github/workflows/")
        or "\\" in workflow
        or PurePosixPath(workflow).as_posix() != workflow
        or any(part in {"", ".", ".."} for part in PurePosixPath(workflow).parts)
        or PurePosixPath(workflow).suffix not in {".yml", ".yaml"}
    ):
        raise VerificationError(
            f"condition {condition_id} has invalid workflow path: {value}"
        )
    return workflow


def condition_attestations(
    condition: dict[str, Any],
) -> list[dict[str, Any]]:
    condition_id = condition["id"]
    raw = condition.get("required_attestations")
    if not isinstance(raw, list) or not raw:
        raise VerificationError(
            f"condition {condition_id} must declare required_attestations"
        )

    result: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str, tuple[str, ...]]] = set()
    for entry in raw:
        if not isinstance(entry, dict):
            raise VerificationError(
                f"condition {condition_id} required_attestations entries must be objects"
            )
        claim = entry.get("claim")
        artifact = entry.get("artifact")
        events = entry.get("events")
        if not isinstance(claim, str) or not claim.strip():
            raise VerificationError(
                f"condition {condition_id} attestation claim must be non-empty"
            )
        if (
            not isinstance(artifact, str)
            or not artifact.strip()
            or artifact != artifact.strip()
            or any(ord(character) < 0x20 for character in artifact)
        ):
            raise VerificationError(
                f"condition {condition_id} attestation artifact must be an exact name"
            )
        if (
            not isinstance(events, list)
            or not events
            or any(not isinstance(event, str) or not event for event in events)
            or len(set(events)) != len(events)
        ):
            raise VerificationError(
                f"condition {condition_id} attestation events must be unique names"
            )
        workflow = normalize_workflow_path(entry.get("workflow"), condition_id)
        normalized = {
            "claim": claim.strip(),
            "workflow": workflow,
            "artifact": artifact,
            "events": list(events),
        }
        identity = (
            normalized["claim"],
            normalized["workflow"],
            normalized["artifact"],
            tuple(normalized["events"]),
        )
        if identity in seen:
            raise VerificationError(
                f"condition {condition_id} has a duplicate required attestation"
            )
        seen.add(identity)
        result.append(normalized)

    has_performance_final = any(
        entry["claim"] == PERFORMANCE_FINAL_CLAIM for entry in result
    )
    if condition["assessment"] == "comparison" and not has_performance_final:
        raise VerificationError(
            f"comparison condition {condition_id} must require "
            f"{PERFORMANCE_FINAL_CLAIM}"
        )
    if condition["assessment"] != "comparison" and has_performance_final:
        raise VerificationError(
            f"release-evidence condition {condition_id} cannot use "
            f"{PERFORMANCE_FINAL_CLAIM}"
        )
    return result


def safe_evidence_path(value: Any, condition_id: str) -> str:
    if not isinstance(value, str) or not value:
        raise VerificationError(
            f"condition {condition_id} tracked evidence path must be non-empty"
        )
    path = PurePosixPath(value)
    if (
        not path.parts
        or path.is_absolute()
        or "\\" in value
        or path.as_posix() != value
        or any(part in {"", ".", ".."} for part in path.parts)
        or any(ord(character) < 0x20 for character in value)
        or ":" in path.parts[0]
    ):
        raise VerificationError(
            f"condition {condition_id} tracked evidence escapes the repository: {value}"
        )
    return value


def tracked_evidence(
    condition: dict[str, Any],
    revision: str,
    cache: dict[str, str],
) -> list[dict[str, str]]:
    condition_id = condition["id"]
    raw_paths = condition.get("evidence")
    if not isinstance(raw_paths, list) or not raw_paths:
        raise VerificationError(
            f"condition {condition_id} must declare tracked evidence paths"
        )
    result: list[dict[str, str]] = []
    seen: set[str] = set()
    for raw_path in raw_paths:
        path = safe_evidence_path(raw_path, condition_id)
        if path in seen:
            raise VerificationError(
                f"condition {condition_id} repeats tracked evidence path {path}"
            )
        seen.add(path)
        digest = cache.get(path)
        if digest is None:
            digest = sha256_bytes(git_bytes("show", f"{revision}:{path}"))
            cache[path] = digest
        result.append({"path": path, "sha256": digest})
    return result


def repository_api_path(owner: str, repository: str) -> str:
    return (
        f"/repos/{urllib.parse.quote(owner, safe='')}/"
        f"{urllib.parse.quote(repository, safe='')}"
    )


def list_artifacts(
    client: GitHubClient,
    repository_path: str,
    artifact_name: str,
) -> list[dict[str, Any]]:
    artifacts: list[dict[str, Any]] = []
    expected_total: int | None = None
    page = 1
    while True:
        payload = client.get_json(
            f"{repository_path}/actions/artifacts",
            {
                "name": artifact_name,
                "per_page": 100,
                "page": page,
            },
        )
        total_count = payload.get("total_count")
        raw_artifacts = payload.get("artifacts")
        if (
            not isinstance(total_count, int)
            or isinstance(total_count, bool)
            or total_count < 0
            or not isinstance(raw_artifacts, list)
        ):
            raise VerificationError(
                f"GitHub artifact listing is malformed for {artifact_name}"
            )
        if expected_total is None:
            expected_total = total_count
        elif total_count != expected_total:
            raise VerificationError(
                f"GitHub artifact listing changed while reading {artifact_name}"
            )
        for artifact in raw_artifacts:
            if not isinstance(artifact, dict):
                raise VerificationError(
                    f"GitHub artifact listing contains a non-object for {artifact_name}"
                )
            artifacts.append(artifact)
        if len(artifacts) > total_count:
            raise VerificationError(
                f"GitHub artifact listing exceeds total_count for {artifact_name}"
            )
        if len(artifacts) >= total_count:
            break
        if not raw_artifacts:
            raise VerificationError(
                f"GitHub artifact listing ended before total_count for {artifact_name}"
            )
        page += 1
        if page > 100:
            raise VerificationError(
                f"GitHub artifact listing is unexpectedly large for {artifact_name}"
            )
    return artifacts


def workflow_run_identity(
    value: Any,
    label: str,
    revision: str,
) -> tuple[int, str]:
    if not isinstance(value, dict):
        raise VerificationError(f"{label} has no workflow_run identity")
    run_id = positive_integer(value.get("id"), f"{label} workflow_run.id")
    head_sha = value.get("head_sha")
    if not isinstance(head_sha, str) or head_sha.lower() != revision.lower():
        raise VerificationError(f"{label} workflow_run.head_sha does not match HEAD")
    return run_id, head_sha


def archive_files(value: bytes, artifact_name: str) -> dict[str, bytes]:
    try:
        with zipfile.ZipFile(io.BytesIO(value)) as archive:
            files: dict[str, bytes] = {}
            total_uncompressed = 0
            for info in archive.infolist():
                if info.is_dir():
                    continue
                if info.filename in files:
                    raise VerificationError(
                        f"artifact {artifact_name} contains duplicate ZIP entry "
                        f"{info.filename}"
                    )
                if info.flag_bits & 0x1:
                    raise VerificationError(
                        f"artifact {artifact_name} contains encrypted ZIP entries"
                    )
                total_uncompressed += info.file_size
                if total_uncompressed > MAX_ARCHIVE_BYTES:
                    raise VerificationError(
                        f"artifact {artifact_name} expands beyond "
                        f"{MAX_ARCHIVE_BYTES} bytes"
                    )
                files[info.filename] = archive.read(info)
    except VerificationError:
        raise
    except (OSError, RuntimeError, zipfile.BadZipFile, zipfile.LargeZipFile) as exc:
        raise VerificationError(f"artifact {artifact_name} is not a valid ZIP: {exc}") from exc
    return files


def unique_root_file(
    files: dict[str, bytes],
    filename: str,
    artifact_name: str,
) -> bytes:
    matches = [name for name in files if PurePosixPath(name).name == filename]
    if matches != [filename]:
        raise VerificationError(
            f"artifact {artifact_name} must contain exactly one root {filename}"
        )
    return files[filename]


def attestation_claims(
    files: dict[str, bytes],
    artifact_name: str,
    revision: str,
) -> frozenset[tuple[str, str]]:
    attestation = load_json_bytes(
        unique_root_file(files, "score-attestation.json", artifact_name),
        f"artifact {artifact_name} score-attestation.json",
    )
    if attestation.get("schema_version") != 1:
        raise VerificationError(
            f"artifact {artifact_name} attestation schema_version must be 1"
        )
    candidate_sha = attestation.get("candidate_git_sha")
    if not isinstance(candidate_sha, str) or candidate_sha.lower() != revision.lower():
        raise VerificationError(
            f"artifact {artifact_name} attestation candidate_git_sha does not match HEAD"
        )
    raw_claims = attestation.get("claims")
    if not isinstance(raw_claims, list) or not raw_claims:
        raise VerificationError(f"artifact {artifact_name} attestation has no claims")

    claims: set[tuple[str, str]] = set()
    for claim in raw_claims:
        if not isinstance(claim, dict):
            raise VerificationError(
                f"artifact {artifact_name} attestation claims must be objects"
            )
        condition_id = claim.get("id")
        claim_name = claim.get("claim")
        if (
            not isinstance(condition_id, str)
            or not condition_id
            or not isinstance(claim_name, str)
            or not claim_name
            or claim.get("status") != "passed"
        ):
            raise VerificationError(
                f"artifact {artifact_name} claims require id, claim, and passed status"
            )
        identity = (condition_id, claim_name)
        if identity in claims:
            raise VerificationError(
                f"artifact {artifact_name} repeats claim {condition_id}/{claim_name}"
            )
        claims.add(identity)
    return frozenset(claims)


def fetch_artifact(
    client: GitHubClient,
    owner: str,
    repository: str,
    repository_full_name: str,
    artifact_name: str,
    revision: str,
) -> VerifiedArtifact:
    repository_path = repository_api_path(owner, repository)
    listed = list_artifacts(client, repository_path, artifact_name)
    candidates: list[dict[str, Any]] = []
    for artifact in listed:
        workflow_run = artifact.get("workflow_run")
        if (
            artifact.get("name") == artifact_name
            and isinstance(workflow_run, dict)
            and isinstance(workflow_run.get("head_sha"), str)
            and workflow_run["head_sha"].lower() == revision.lower()
        ):
            candidates.append(artifact)
    if len(candidates) != 1:
        raise VerificationError(
            f"expected exactly one artifact named {artifact_name} for HEAD; "
            f"found {len(candidates)}"
        )

    listed_artifact = candidates[0]
    artifact_id = positive_integer(
        listed_artifact.get("id"),
        f"artifact {artifact_name} id",
    )
    if listed_artifact.get("expired") is not False:
        raise VerificationError(f"artifact {artifact_name} is expired")
    listed_digest = immutable_digest(
        listed_artifact.get("digest"),
        f"artifact {artifact_name}",
    )
    listed_run_id, _ = workflow_run_identity(
        listed_artifact.get("workflow_run"),
        f"artifact {artifact_name}",
        revision,
    )

    artifact_path = f"{repository_path}/actions/artifacts/{artifact_id}"
    metadata = client.get_json(artifact_path)
    if positive_integer(metadata.get("id"), f"artifact {artifact_name} metadata id") != artifact_id:
        raise VerificationError(f"artifact {artifact_name} metadata id changed")
    if metadata.get("name") != artifact_name:
        raise VerificationError(f"artifact {artifact_name} metadata name changed")
    if metadata.get("expired") is not False:
        raise VerificationError(f"artifact {artifact_name} metadata is expired")
    metadata_digest = immutable_digest(
        metadata.get("digest"),
        f"artifact {artifact_name} metadata",
    )
    if metadata_digest != listed_digest:
        raise VerificationError(f"artifact {artifact_name} digest changed after listing")
    metadata_run_id, _ = workflow_run_identity(
        metadata.get("workflow_run"),
        f"artifact {artifact_name} metadata",
        revision,
    )
    if metadata_run_id != listed_run_id:
        raise VerificationError(f"artifact {artifact_name} workflow run changed")

    download_path = f"{artifact_path}/zip"
    expected_download_url = client.url(download_path)
    if metadata.get("archive_download_url") != expected_download_url:
        raise VerificationError(
            f"artifact {artifact_name} archive_download_url is not the exact GitHub API URL"
        )

    run = client.get_json(f"{repository_path}/actions/runs/{listed_run_id}")
    if positive_integer(run.get("id"), f"workflow run {listed_run_id} id") != listed_run_id:
        raise VerificationError(f"workflow run {listed_run_id} id changed")
    if run.get("status") != "completed" or run.get("conclusion") != "success":
        raise VerificationError(
            f"workflow run {listed_run_id} is not completed successfully"
        )
    run_head_sha = run.get("head_sha")
    if not isinstance(run_head_sha, str) or run_head_sha.lower() != revision.lower():
        raise VerificationError(f"workflow run {listed_run_id} head_sha does not match HEAD")
    run_path = run.get("path")
    run_event = run.get("event")
    if not isinstance(run_path, str) or not run_path:
        raise VerificationError(f"workflow run {listed_run_id} has no workflow path")
    if not isinstance(run_event, str) or not run_event:
        raise VerificationError(f"workflow run {listed_run_id} has no event")
    run_repository = run.get("repository")
    if (
        not isinstance(run_repository, dict)
        or not isinstance(run_repository.get("full_name"), str)
        or run_repository["full_name"].casefold() != repository_full_name.casefold()
    ):
        raise VerificationError(
            f"workflow run {listed_run_id} repository does not match "
            f"{repository_full_name}"
        )

    archive = client.get_bytes(download_path)
    archive_digest = sha256_bytes(archive)
    if archive_digest != metadata_digest:
        raise VerificationError(
            f"artifact {artifact_name} downloaded ZIP digest does not match GitHub"
        )
    files = archive_files(archive, artifact_name)
    claims = attestation_claims(files, artifact_name, revision)
    return VerifiedArtifact(
        name=artifact_name,
        artifact_id=artifact_id,
        digest=metadata_digest,
        run_id=listed_run_id,
        run_path=run_path,
        run_event=run_event,
        files=files,
        claims=claims,
    )


def validated_git_object(value: Any, label: str) -> str:
    if not isinstance(value, str) or GIT_OBJECT.fullmatch(value) is None:
        raise VerificationError(f"{label} is not a valid Git object id")
    if set(value.lower()) == {"0"}:
        raise VerificationError(f"{label} cannot be the all-zero Git object id")
    return value


def exact_nonempty_string(value: Any, label: str) -> str:
    if (
        not isinstance(value, str)
        or not value
        or value != value.strip()
        or any(ord(character) < 0x20 for character in value)
    ):
        raise VerificationError(f"{label} must be an exact non-empty string")
    return value


def root_artifact_filename(value: Any, label: str) -> str:
    filename = exact_nonempty_string(value, label)
    path = PurePosixPath(filename)
    if (
        path.name != filename
        or filename in {".", ".."}
        or "/" in filename
        or "\\" in filename
        or ":" in filename
    ):
        raise VerificationError(f"{label} must be a safe root filename")
    return filename


def sha256_hex(value: Any, label: str) -> str:
    if not isinstance(value, str) or SHA256_HEX.fullmatch(value) is None:
        raise VerificationError(f"{label} must be a lowercase SHA-256 digest")
    if value == "0" * 64:
        raise VerificationError(f"{label} cannot be the all-zero SHA-256 digest")
    return value


def finite_number(value: Any, label: str, *, allow_zero: bool = False) -> float:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        qualifier = "non-negative" if allow_zero else "positive"
        raise VerificationError(f"{label} must be a finite {qualifier} number")
    try:
        number = float(value)
    except (OverflowError, ValueError) as exc:
        raise VerificationError(f"{label} must be a finite number") from exc
    if (
        not math.isfinite(number)
        or number < 0
        or (number == 0 and not allow_zero)
    ):
        qualifier = "non-negative" if allow_zero else "positive"
        raise VerificationError(f"{label} must be a finite {qualifier} number")
    return number


def validate_ci_platform_policy(manifest: dict[str, Any]) -> dict[str, Any]:
    policy = manifest.get("ci_platform_policy")
    if policy != LOCKED_CI_PLATFORM_POLICY:
        raise VerificationError(
            "manifest ci_platform_policy must match the locked "
            "hosted-platforms-and-i686-layout-v2 contract"
        )
    return policy


def validate_safety_stable_policy(manifest: dict[str, Any]) -> dict[str, Any]:
    policy = manifest.get("safety_stable_policy")
    if policy != LOCKED_SAFETY_STABLE_POLICY:
        raise VerificationError(
            "manifest safety_stable_policy must match the locked "
            "release-safety-stable-v1 contract"
        )
    return policy


def score_attestation_payload(artifact: VerifiedArtifact) -> dict[str, Any]:
    return load_json_bytes(
        unique_root_file(
            artifact.files,
            "score-attestation.json",
            artifact.name,
        ),
        f"artifact {artifact.name} score-attestation.json",
    )


def validate_ci_platforms(
    artifact: VerifiedArtifact,
    policy: dict[str, Any],
    revision: str,
) -> dict[str, Any]:
    if (
        artifact.run_path != policy["workflow"]
        and not artifact.run_path.startswith(policy["workflow"] + "@")
    ):
        raise VerificationError(
            f"artifact {artifact.name} CI platform workflow does not match policy"
        )
    attestation = score_attestation_payload(artifact)
    label = f"artifact {artifact.name} CI platform evidence"
    if attestation.get("ci_platform_policy_id") != policy["policy_id"]:
        raise VerificationError(f"{label} policy_id does not match manifest")
    if attestation.get("workflow_needs") != policy["required_workflow_needs"]:
        raise VerificationError(
            f"{label} workflow_needs do not lock deterministic, MSRV, "
            "hosted platforms, and i686 layout"
        )
    raw_platforms = attestation.get("platform_files")
    if not isinstance(raw_platforms, list):
        raise VerificationError(f"{label} platform_files must be an array")
    expected_platforms = {
        entry["matrix_os"]: entry for entry in policy["platforms"]
    }
    if len(raw_platforms) != len(expected_platforms):
        raise VerificationError(
            f"{label} must contain exactly {len(expected_platforms)} platforms"
        )
    expected_checks = {
        entry["id"]: entry for entry in policy["required_checks"]
    }
    seen: set[str] = set()
    provenance: list[dict[str, Any]] = []
    for entry in raw_platforms:
        if not isinstance(entry, dict) or set(entry) != {
            "matrix_os",
            "descriptor",
            "logs",
        }:
            raise VerificationError(
                f"{label} platform entries must contain matrix_os, descriptor, and logs"
            )
        matrix_os = exact_nonempty_string(
            entry.get("matrix_os"),
            f"{label} matrix_os",
        )
        platform = expected_platforms.get(matrix_os)
        if platform is None or matrix_os in seen:
            raise VerificationError(f"{label} has unexpected platform {matrix_os}")
        seen.add(matrix_os)

        descriptor_filename = f"platform-{matrix_os}.json"
        descriptor_provenance = validate_raw_artifact(
            artifact.files,
            entry.get("descriptor"),
            descriptor_filename,
            artifact.name,
            f"{label} {matrix_os} descriptor",
        )
        descriptor_raw = artifact.files[descriptor_filename]
        descriptor = load_json_bytes(
            descriptor_raw,
            f"{label} {matrix_os} descriptor",
        )
        if set(descriptor) != {
            "schema_version",
            "candidate_git_sha",
            "matrix_os",
            "runner_os",
            "runner_arch",
            "rustc",
            "checks",
            "log_sha256",
        }:
            raise VerificationError(
                f"{label} {matrix_os} descriptor has unexpected fields"
            )
        if descriptor.get("schema_version") != policy["descriptor_schema_version"]:
            raise VerificationError(
                f"{label} {matrix_os} descriptor schema_version does not match policy"
            )
        candidate_sha = validated_git_object(
            descriptor.get("candidate_git_sha"),
            f"{label} {matrix_os} candidate_git_sha",
        )
        if candidate_sha.lower() != revision.lower():
            raise VerificationError(
                f"{label} {matrix_os} candidate_git_sha does not match HEAD"
            )
        for field in ("matrix_os", "runner_os", "runner_arch"):
            if descriptor.get(field) != platform[field]:
                raise VerificationError(
                    f"{label} {matrix_os} {field} does not match policy"
                )
        rustc = descriptor.get("rustc")
        if (
            not isinstance(rustc, str)
            or not rustc
            or rustc != rustc.strip()
            or len(rustc.encode("utf-8")) > 16 * 1024
            or any(character == "\x00" for character in rustc)
            or not rustc.splitlines()
            or not rustc.splitlines()[0].startswith("rustc ")
            or f"host: {platform['rustc_host']}" not in rustc.splitlines()
        ):
            raise VerificationError(
                f"{label} {matrix_os} rustc identity does not match policy host"
            )
        expected_statuses = {
            check_id: "passed" for check_id in expected_checks
        }
        if descriptor.get("checks") != expected_statuses:
            raise VerificationError(
                f"{label} {matrix_os} checks are not all passed"
            )
        log_sha256 = descriptor.get("log_sha256")
        expected_source_logs = {
            check["log_filename"] for check in expected_checks.values()
        }
        if (
            not isinstance(log_sha256, dict)
            or set(log_sha256) != expected_source_logs
        ):
            raise VerificationError(
                f"{label} {matrix_os} descriptor log digests do not match policy"
            )
        logs = entry.get("logs")
        if not isinstance(logs, dict) or set(logs) != set(expected_checks):
            raise VerificationError(
                f"{label} {matrix_os} logs do not match required checks"
            )
        log_provenance: dict[str, dict[str, str]] = {}
        for check_id, check in expected_checks.items():
            source_filename = check["log_filename"]
            score_filename = f"platform-{matrix_os}-{source_filename}"
            raw_provenance = validate_raw_artifact(
                artifact.files,
                logs[check_id],
                score_filename,
                artifact.name,
                f"{label} {matrix_os} {check_id} log",
            )
            descriptor_digest = sha256_hex(
                log_sha256.get(source_filename),
                f"{label} {matrix_os} descriptor {source_filename} digest",
            )
            if descriptor_digest != raw_provenance["sha256"]:
                raise VerificationError(
                    f"{label} {matrix_os} {check_id} descriptor digest "
                    "does not match raw log"
                )
            raw = artifact.files[score_filename]
            markers = (
                f"candidate_git_sha={revision}\n".encode(),
                f"matrix_os={matrix_os}\n".encode(),
                f"runner_os={platform['runner_os']}\n".encode(),
                f"runner_arch={platform['runner_arch']}\n".encode(),
                check["passing_marker"].encode(),
            )
            if any(marker not in raw for marker in markers):
                raise VerificationError(
                    f"{label} {matrix_os} {check_id} log is missing "
                    "candidate, platform, or passing markers"
                )
            log_provenance[check_id] = raw_provenance
        provenance.append(
            {
                "matrix_os": matrix_os,
                "runner_os": platform["runner_os"],
                "runner_arch": platform["runner_arch"],
                "rustc": rustc,
                "descriptor": descriptor_provenance,
                "logs": log_provenance,
            }
        )
    if seen != set(expected_platforms):
        raise VerificationError(f"{label} is missing a required hosted platform")
    return {
        "policy_id": policy["policy_id"],
        "workflow_needs": list(policy["required_workflow_needs"]),
        "layout_gate": dict(policy["layout_gate"]),
        "platforms": provenance,
    }


def validate_safety_stable(
    artifact: VerifiedArtifact,
    policy: dict[str, Any],
    revision: str,
) -> dict[str, Any]:
    if (
        artifact.run_path != policy["workflow"]
        and not artifact.run_path.startswith(policy["workflow"] + "@")
    ):
        raise VerificationError(
            f"artifact {artifact.name} stable safety workflow does not match policy"
        )
    attestation = score_attestation_payload(artifact)
    label = f"artifact {artifact.name} stable safety evidence"
    if attestation.get("safety_stable_policy_id") != policy["policy_id"]:
        raise VerificationError(f"{label} policy_id does not match manifest")
    if attestation.get("workflow_needs") != policy["required_workflow_needs"]:
        raise VerificationError(
            f"{label} workflow_needs do not lock Miri, ASan, and fuzz"
        )
    raw_files = attestation.get("safety_files")
    if not isinstance(raw_files, list):
        raise VerificationError(f"{label} safety_files must be an array")
    expected = {entry["id"]: entry for entry in policy["required_logs"]}
    if len(raw_files) != len(expected):
        raise VerificationError(
            f"{label} must contain exactly {len(expected)} required logs"
        )
    seen: set[str] = set()
    provenance: list[dict[str, Any]] = []
    for entry in raw_files:
        if not isinstance(entry, dict):
            raise VerificationError(f"{label} safety_files entries must be objects")
        evidence_id = exact_nonempty_string(
            entry.get("id"),
            f"{label} safety_files id",
        )
        required = expected.get(evidence_id)
        if required is None or evidence_id in seen:
            raise VerificationError(
                f"{label} has unexpected safety evidence {evidence_id}"
            )
        seen.add(evidence_id)
        required_keys = {"id", "kind", "filename", "sha256"}
        if required["kind"] == "fuzz":
            required_keys.add("max_total_time_seconds")
        if set(entry) != required_keys:
            raise VerificationError(
                f"{label} {evidence_id} descriptor fields do not match policy"
            )
        if entry.get("kind") != required["kind"]:
            raise VerificationError(
                f"{label} {evidence_id} kind does not match policy"
            )
        raw_provenance = validate_raw_artifact(
            artifact.files,
            {
                "filename": entry.get("filename"),
                "sha256": entry.get("sha256"),
            },
            required["filename"],
            artifact.name,
            f"{label} {evidence_id} log",
        )
        raw = artifact.files[required["filename"]]
        markers = (
            f"candidate_git_sha={revision}\n".encode(),
            f"target={required['target']}\n".encode(),
            policy["passing_marker"].encode(),
        )
        if any(marker not in raw for marker in markers):
            raise VerificationError(
                f"{label} {evidence_id} log is missing candidate, target, "
                "or passing markers"
            )
        record: dict[str, Any] = {
            "id": evidence_id,
            "kind": required["kind"],
            **raw_provenance,
        }
        if required["kind"] == "test":
            if required["test_marker"].encode() not in raw:
                raise VerificationError(
                    f"{label} {evidence_id} log has no passing test marker"
                )
        else:
            duration = positive_integer(
                entry.get("max_total_time_seconds"),
                f"{label} {evidence_id} max_total_time_seconds",
            )
            match = re.search(rb"(?m)^max_total_time=([0-9]+)\r?$", raw)
            if match is None:
                raise VerificationError(
                    f"{label} {evidence_id} log has no fuzz duration marker"
                )
            logged_duration = int(match.group(1))
            if duration != logged_duration:
                raise VerificationError(
                    f"{label} {evidence_id} duration does not match raw log"
                )
            if duration < policy["minimum_fuzz_seconds"]:
                raise VerificationError(
                    f"{label} {evidence_id} duration is below "
                    f"{policy['minimum_fuzz_seconds']} seconds"
                )
            record["max_total_time_seconds"] = duration
        provenance.append(record)
    if seen != set(expected):
        raise VerificationError(f"{label} is missing a required safety log")
    return {
        "policy_id": policy["policy_id"],
        "workflow_needs": list(policy["required_workflow_needs"]),
        "files": provenance,
    }


def validate_downstream_policy(manifest: dict[str, Any]) -> dict[str, Any]:
    policy = manifest.get("downstream_traffic_policy")
    if not isinstance(policy, dict):
        raise VerificationError(
            "manifest downstream_traffic_policy object is required for "
            f"{DOWNSTREAM_TRAFFIC_CLAIM}"
        )
    policy_digest = sha256_bytes(canonical_json_bytes(policy))
    if policy_digest != LOCKED_DOWNSTREAM_TRAFFIC_POLICY_SHA256:
        raise VerificationError(
            "manifest downstream_traffic_policy does not match the locked "
            "reviewed policy"
        )
    if policy.get("schema_version") != 2:
        raise VerificationError(
            "manifest downstream_traffic_policy.schema_version must be 2"
        )
    exact_nonempty_string(
        policy.get("policy_id"),
        "manifest downstream_traffic_policy.policy_id",
    )
    if policy.get("evidence_filename") != DOWNSTREAM_TRAFFIC_FILENAME:
        raise VerificationError(
            "manifest downstream_traffic_policy.evidence_filename must be "
            f"{DOWNSTREAM_TRAFFIC_FILENAME}"
        )

    crate = policy.get("crate")
    if not isinstance(crate, dict):
        raise VerificationError(
            "manifest downstream_traffic_policy.crate object is missing"
        )
    crate_name = exact_nonempty_string(
        crate.get("name"),
        "manifest downstream_traffic_policy.crate.name",
    )
    if crate_name != manifest.get("crate"):
        raise VerificationError(
            "manifest downstream traffic crate name does not match manifest crate"
        )
    crate_version = exact_nonempty_string(
        crate.get("version"),
        "manifest downstream_traffic_policy.crate.version",
    )
    package_filename = root_artifact_filename(
        crate.get("package_filename"),
        "manifest downstream_traffic_policy.crate.package_filename",
    )
    if package_filename != f"{crate_name}-{crate_version}.crate":
        raise VerificationError(
            "manifest downstream traffic package filename does not match "
            "crate name and version"
        )

    downstream = policy.get("downstream")
    if not isinstance(downstream, dict):
        raise VerificationError(
            "manifest downstream_traffic_policy.downstream object is missing"
        )
    downstream_repository = exact_nonempty_string(
        downstream.get("repository"),
        "manifest downstream_traffic_policy.downstream.repository",
    )
    if GITHUB_REPOSITORY.fullmatch(downstream_repository) is None:
        raise VerificationError(
            "manifest downstream traffic repository must use OWNER/REPOSITORY"
        )
    validated_git_object(
        downstream.get("source_git_sha"),
        "manifest downstream_traffic_policy.downstream.source_git_sha",
    )
    exact_nonempty_string(
        policy.get("workload_id"),
        "manifest downstream_traffic_policy.workload_id",
    )

    required_inputs = policy.get("required_input_artifacts")
    if (
        not isinstance(required_inputs, dict)
        or set(required_inputs) != REQUIRED_DOWNSTREAM_INPUTS
    ):
        raise VerificationError(
            "manifest downstream traffic required_input_artifacts must declare "
            "Cargo.lock, dependency graph, build config, dataset summary, and "
            "package matrix roles"
        )
    input_filenames = [
        root_artifact_filename(
            required_inputs[role],
            f"manifest downstream traffic input {role}",
        )
        for role in sorted(REQUIRED_DOWNSTREAM_INPUTS)
    ]
    if len(set(input_filenames)) != len(input_filenames):
        raise VerificationError(
            "manifest downstream traffic input artifact filenames must be unique"
        )

    package_matrix = policy.get("package_matrix")
    if not isinstance(package_matrix, dict):
        raise VerificationError(
            "manifest downstream_traffic_policy.package_matrix object is missing"
        )
    workspace_minimum = positive_integer(
        package_matrix.get("minimum_workspace_packages"),
        "manifest downstream traffic minimum_workspace_packages",
    )
    standalone_minimum = positive_integer(
        package_matrix.get("minimum_standalone_packages"),
        "manifest downstream traffic minimum_standalone_packages",
    )
    if workspace_minimum < 20 or standalone_minimum < 2:
        raise VerificationError(
            "manifest downstream package minima cannot be weaker than "
            "20 workspace and 2 standalone packages"
        )
    raw_profiles = package_matrix.get("required_crate_feature_profiles")
    if (
        not isinstance(raw_profiles, list)
        or not raw_profiles
        or any(
            not isinstance(profile, str)
            or not profile
            or profile != profile.strip()
            or any(ord(character) < 0x20 for character in profile)
            for profile in raw_profiles
        )
        or len(set(raw_profiles)) != len(raw_profiles)
    ):
        raise VerificationError(
            "manifest downstream required crate feature profiles must be "
            "unique exact names"
        )

    measurement = policy.get("measurement")
    if not isinstance(measurement, dict):
        raise VerificationError(
            "manifest downstream_traffic_policy.measurement object is missing"
        )
    positive_integer(
        measurement.get("minimum_warmup_seconds"),
        "manifest downstream traffic minimum_warmup_seconds",
    )
    positive_integer(
        measurement.get("minimum_duration_seconds"),
        "manifest downstream traffic minimum_duration_seconds",
    )
    positive_integer(
        measurement.get("minimum_samples_per_round"),
        "manifest downstream traffic minimum_samples_per_round",
    )
    minimum_rounds = positive_integer(
        measurement.get("minimum_rounds"),
        "manifest downstream traffic minimum_rounds",
    )
    if minimum_rounds < 2:
        raise VerificationError(
            "manifest downstream traffic requires at least two independent rounds"
        )
    if measurement.get("round_artifact_filename_pattern") != "traffic-r{round}.json":
        raise VerificationError(
            "manifest downstream traffic round artifact filename pattern must be "
            "traffic-r{round}.json"
        )
    if measurement.get("round_schema_version") != DOWNSTREAM_ROUND_SCHEMA_VERSION:
        raise VerificationError(
            "manifest downstream traffic round_schema_version does not match "
            "the verifier"
        )
    if measurement.get("require_non_overlapping_windows") is not True:
        raise VerificationError(
            "manifest downstream traffic must require non-overlapping windows"
        )
    if measurement.get("require_distinct_semantic_samples") is not True:
        raise VerificationError(
            "manifest downstream traffic must require distinct semantic samples"
        )
    thresholds = measurement.get("metric_thresholds")
    if not isinstance(thresholds, dict) or set(thresholds) != DOWNSTREAM_METRICS:
        raise VerificationError(
            "manifest downstream traffic metric_thresholds must declare the "
            "exact throughput, P50/P95/P99, CPU, RSS, and allocation metrics"
        )
    for metric in sorted(DOWNSTREAM_METRICS):
        threshold = thresholds[metric]
        if not isinstance(threshold, dict):
            raise VerificationError(
                f"manifest downstream traffic threshold {metric} must be an object"
            )
        if metric in DOWNSTREAM_HIGHER_IS_BETTER:
            key = "minimum_candidate_to_baseline_ratio"
            if set(threshold) != {key}:
                raise VerificationError(
                    f"manifest downstream traffic threshold {metric} must use {key}"
                )
            ratio = finite_number(
                threshold.get(key),
                f"manifest downstream traffic threshold {metric}.{key}",
            )
            if ratio < 0.97:
                raise VerificationError(
                    "manifest downstream throughput threshold cannot allow "
                    "more than 3% regression"
                )
        else:
            key = "maximum_candidate_to_baseline_ratio"
            if set(threshold) != {key}:
                raise VerificationError(
                    f"manifest downstream traffic threshold {metric} must use {key}"
                )
            ratio = finite_number(
                threshold.get(key),
                f"manifest downstream traffic threshold {metric}.{key}",
            )
            if ratio > 1.03:
                raise VerificationError(
                    f"manifest downstream {metric} threshold cannot allow "
                    "more than 3% regression"
                )
    return policy


def validate_raw_artifact(
    files: dict[str, bytes],
    descriptor: Any,
    expected_filename: str,
    artifact_name: str,
    label: str,
) -> dict[str, str]:
    if not isinstance(descriptor, dict) or set(descriptor) != {
        "filename",
        "sha256",
    }:
        raise VerificationError(
            f"{label} must contain exactly filename and sha256"
        )
    filename = root_artifact_filename(descriptor.get("filename"), f"{label}.filename")
    if filename != expected_filename:
        raise VerificationError(
            f"{label}.filename must be {expected_filename}"
        )
    expected_digest = sha256_hex(descriptor.get("sha256"), f"{label}.sha256")
    raw = unique_root_file(files, filename, artifact_name)
    actual_digest = sha256_bytes(raw)
    if actual_digest != expected_digest:
        raise VerificationError(f"{label} SHA-256 does not match artifact content")
    return {"filename": filename, "sha256": actual_digest}


def validate_crate_package_vcs(
    package: bytes,
    package_filename: str,
    crate_name: str,
    crate_version: str,
    revision: str,
    label: str,
) -> None:
    expected_root = f"{crate_name}-{crate_version}"
    expected_vcs_path = f"{expected_root}/.cargo_vcs_info.json"
    try:
        with tarfile.open(fileobj=io.BytesIO(package), mode="r:gz") as archive:
            matches: list[tarfile.TarInfo] = []
            total_uncompressed = 0
            for member in archive.getmembers():
                path = PurePosixPath(member.name)
                if (
                    not path.parts
                    or path.is_absolute()
                    or any(part in {"", ".", ".."} for part in path.parts)
                    or path.parts[0] != expected_root
                ):
                    raise VerificationError(
                        f"{label} contains a path outside {expected_root}"
                    )
                total_uncompressed += member.size
                if total_uncompressed > MAX_ARCHIVE_BYTES:
                    raise VerificationError(
                        f"{label} expands beyond {MAX_ARCHIVE_BYTES} bytes"
                    )
                if member.name == expected_vcs_path:
                    matches.append(member)
            if len(matches) != 1 or not matches[0].isfile():
                raise VerificationError(
                    f"{label} must contain exactly one {expected_vcs_path}"
                )
            extracted = archive.extractfile(matches[0])
            if extracted is None:
                raise VerificationError(f"{label} cannot read {expected_vcs_path}")
            vcs_raw = extracted.read(MAX_JSON_BYTES + 1)
    except VerificationError:
        raise
    except (OSError, tarfile.TarError) as exc:
        raise VerificationError(
            f"{label} {package_filename} is not a valid Cargo crate archive: {exc}"
        ) from exc
    vcs = load_json_bytes(vcs_raw, f"{label} {expected_vcs_path}")
    git = vcs.get("git")
    if not isinstance(git, dict):
        raise VerificationError(f"{label} has no Cargo VCS git identity")
    package_sha = validated_git_object(
        git.get("sha1"),
        f"{label} Cargo VCS git.sha1",
    )
    if package_sha.lower() != revision.lower():
        raise VerificationError(
            f"{label} Cargo package VCS SHA does not match exact candidate"
        )
    dirty = git.get("dirty")
    if dirty is not None and dirty is not False:
        raise VerificationError(f"{label} Cargo package was built from a dirty tree")
    if vcs.get("path_in_vcs") != "":
        raise VerificationError(
            f"{label} Cargo package path_in_vcs does not identify the repository root"
        )


def validate_candidate_package(
    path: Path,
    policy: dict[str, Any],
    revision: str,
) -> dict[str, Any]:
    crate = policy["crate"]
    expected_filename = crate["package_filename"]
    if path.name != expected_filename:
        raise VerificationError(
            f"candidate package filename must be {expected_filename}"
        )
    package = read_file_limited(path, MAX_ARCHIVE_BYTES, "candidate package")
    validate_crate_package_vcs(
        package,
        expected_filename,
        crate["name"],
        crate["version"],
        revision,
        "candidate package",
    )
    return {
        "filename": expected_filename,
        "sha256": sha256_bytes(package),
        "size_bytes": len(package),
    }


def validate_downstream_package_matrix(
    evidence: dict[str, Any],
    policy: dict[str, Any],
    label: str,
) -> dict[str, int]:
    raw_feature_matrix = evidence.get("crate_feature_matrix")
    if not isinstance(raw_feature_matrix, list) or not raw_feature_matrix:
        raise VerificationError(f"{label} crate_feature_matrix must be a non-empty array")
    profiles: set[str] = set()
    for entry in raw_feature_matrix:
        if not isinstance(entry, dict):
            raise VerificationError(f"{label} crate feature entries must be objects")
        profile = exact_nonempty_string(
            entry.get("profile"),
            f"{label} crate feature profile",
        )
        if entry.get("status") != "passed":
            raise VerificationError(
                f"{label} crate feature profile {profile} did not pass"
            )
        if profile in profiles:
            raise VerificationError(
                f"{label} repeats crate feature profile {profile}"
            )
        profiles.add(profile)
    required_profiles = set(
        policy["package_matrix"]["required_crate_feature_profiles"]
    )
    missing_profiles = required_profiles - profiles
    if missing_profiles:
        raise VerificationError(
            f"{label} is missing required crate feature profiles: "
            + ", ".join(sorted(missing_profiles))
        )

    raw_matrix = evidence.get("package_matrix")
    if not isinstance(raw_matrix, dict) or set(raw_matrix) != {
        "workspace",
        "standalone",
    }:
        raise VerificationError(
            f"{label} package_matrix must contain workspace and standalone arrays"
        )
    seen_packages: set[str] = set()
    counts: dict[str, int] = {}
    for kind in ("workspace", "standalone"):
        entries = raw_matrix[kind]
        if not isinstance(entries, list):
            raise VerificationError(f"{label} package_matrix.{kind} must be an array")
        for entry in entries:
            if not isinstance(entry, dict):
                raise VerificationError(
                    f"{label} package_matrix.{kind} entries must be objects"
                )
            package = exact_nonempty_string(
                entry.get("package"),
                f"{label} package_matrix.{kind} package",
            )
            if package in seen_packages:
                raise VerificationError(
                    f"{label} repeats downstream package {package}"
                )
            seen_packages.add(package)
            features = entry.get("features")
            if (
                not isinstance(features, list)
                or not features
                or any(
                    not isinstance(feature, str)
                    or not feature
                    or feature != feature.strip()
                    or any(ord(character) < 0x20 for character in feature)
                    for feature in features
                )
                or len(set(features)) != len(features)
            ):
                raise VerificationError(
                    f"{label} package {package} must declare unique exact features"
                )
            if entry.get("status") != "passed":
                raise VerificationError(
                    f"{label} downstream package {package} did not pass"
                )
        counts[kind] = len(entries)
    policy_matrix = policy["package_matrix"]
    if counts["workspace"] < policy_matrix["minimum_workspace_packages"]:
        raise VerificationError(
            f"{label} workspace package count is below policy minimum"
        )
    if counts["standalone"] < policy_matrix["minimum_standalone_packages"]:
        raise VerificationError(
            f"{label} standalone package count is below policy minimum"
        )
    counts["crate_feature_profiles"] = len(profiles)
    return counts


def validate_downstream_metrics(
    metrics: Any,
    thresholds: dict[str, dict[str, Any]],
    label: str,
) -> dict[str, float]:
    if not isinstance(metrics, dict) or set(metrics) != {"baseline", "candidate"}:
        raise VerificationError(
            f"{label} metrics must contain baseline and candidate objects"
        )
    baseline = metrics["baseline"]
    candidate = metrics["candidate"]
    if (
        not isinstance(baseline, dict)
        or set(baseline) != DOWNSTREAM_METRICS
        or not isinstance(candidate, dict)
        or set(candidate) != DOWNSTREAM_METRICS
    ):
        raise VerificationError(
            f"{label} metrics must contain the exact throughput, P50/P95/P99, "
            "CPU, RSS, and allocation fields"
        )
    ratios: dict[str, float] = {}
    for metric in sorted(DOWNSTREAM_METRICS):
        base_value = finite_number(
            baseline[metric],
            f"{label} baseline.{metric}",
        )
        candidate_value = finite_number(
            candidate[metric],
            f"{label} candidate.{metric}",
            allow_zero=metric == "allocations_per_operation",
        )
        ratio = candidate_value / base_value
        ratios[metric] = ratio
        threshold = thresholds[metric]
        if metric in DOWNSTREAM_HIGHER_IS_BETTER:
            minimum = float(threshold["minimum_candidate_to_baseline_ratio"])
            if ratio < minimum:
                raise VerificationError(
                    f"{label} {metric} ratio {ratio:.6f} is below {minimum:.6f}"
                )
        else:
            maximum = float(threshold["maximum_candidate_to_baseline_ratio"])
            if ratio > maximum:
                raise VerificationError(
                    f"{label} {metric} ratio {ratio:.6f} exceeds {maximum:.6f}"
                )
    return ratios


ROUND_SAMPLE_FIELDS = frozenset(
    {
        "sequence",
        "operations",
        "elapsed_ns",
        "latency_ns",
        "cpu_time_ns",
        "rss_peak_bytes",
        "allocations",
    }
)


def nearest_rank(values: list[int], percentile: float) -> int:
    if not values:
        raise VerificationError("cannot calculate a percentile without samples")
    ordered = sorted(values)
    index = max(0, math.ceil(percentile * len(ordered)) - 1)
    return ordered[index]


def metrics_from_round_samples(
    samples: Any,
    expected_count: int,
    label: str,
) -> tuple[dict[str, float], list[dict[str, int]]]:
    if not isinstance(samples, list) or len(samples) != expected_count:
        raise VerificationError(
            f"{label} must contain exactly {expected_count} samples"
        )
    validated: list[dict[str, int]] = []
    for index, sample in enumerate(samples, start=1):
        sample_label = f"{label}[{index}]"
        if not isinstance(sample, dict) or set(sample) != ROUND_SAMPLE_FIELDS:
            raise VerificationError(
                f"{sample_label} must contain the exact raw sample fields"
            )
        if sample.get("sequence") != index:
            raise VerificationError(
                f"{sample_label}.sequence must preserve ordered samples"
            )
        validated.append(
            {
                "sequence": index,
                "operations": bounded_positive_integer(
                    sample.get("operations"),
                    f"{sample_label}.operations",
                ),
                "elapsed_ns": bounded_positive_integer(
                    sample.get("elapsed_ns"),
                    f"{sample_label}.elapsed_ns",
                ),
                "latency_ns": bounded_positive_integer(
                    sample.get("latency_ns"),
                    f"{sample_label}.latency_ns",
                ),
                "cpu_time_ns": bounded_nonnegative_integer(
                    sample.get("cpu_time_ns"),
                    f"{sample_label}.cpu_time_ns",
                ),
                "rss_peak_bytes": bounded_positive_integer(
                    sample.get("rss_peak_bytes"),
                    f"{sample_label}.rss_peak_bytes",
                ),
                "allocations": bounded_nonnegative_integer(
                    sample.get("allocations"),
                    f"{sample_label}.allocations",
                ),
            }
        )

    total_operations = sum(sample["operations"] for sample in validated)
    total_elapsed_ns = sum(sample["elapsed_ns"] for sample in validated)
    latencies = [sample["latency_ns"] for sample in validated]
    return (
        {
            "throughput_ops_per_second": (
                total_operations * 1_000_000_000.0 / total_elapsed_ns
            ),
            "latency_p50_ms": nearest_rank(latencies, 0.50) / 1_000_000.0,
            "latency_p95_ms": nearest_rank(latencies, 0.95) / 1_000_000.0,
            "latency_p99_ms": nearest_rank(latencies, 0.99) / 1_000_000.0,
            "cpu_time_per_operation_ns": (
                sum(sample["cpu_time_ns"] for sample in validated)
                / total_operations
            ),
            "rss_peak_bytes": float(
                max(sample["rss_peak_bytes"] for sample in validated)
            ),
            "allocations_per_operation": (
                sum(sample["allocations"] for sample in validated)
                / total_operations
            ),
        },
        validated,
    )


def validate_reported_round_metrics(
    reported: Any,
    computed: dict[str, dict[str, float]],
    label: str,
) -> None:
    if not isinstance(reported, dict) or set(reported) != {"baseline", "candidate"}:
        raise VerificationError(
            f"{label} metrics must contain baseline and candidate objects"
        )
    for side in ("baseline", "candidate"):
        values = reported.get(side)
        if not isinstance(values, dict) or set(values) != DOWNSTREAM_METRICS:
            raise VerificationError(
                f"{label} {side} metrics do not match the raw metric schema"
            )
        for metric in sorted(DOWNSTREAM_METRICS):
            actual = finite_number(
                values[metric],
                f"{label} {side}.{metric}",
                allow_zero=metric == "allocations_per_operation",
            )
            expected = computed[side][metric]
            if not math.isclose(actual, expected, rel_tol=1e-9, abs_tol=1e-9):
                raise VerificationError(
                    f"{label} {side}.{metric} does not match recomputed raw samples"
                )


def validate_downstream_round_raw(
    raw: bytes,
    *,
    artifact: VerifiedArtifact,
    policy: dict[str, Any],
    revision: str,
    candidate_package_sha256: str,
    input_sha256: dict[str, str],
    round_id: str,
    sample_count: int,
    duration_seconds: int,
    label: str,
) -> dict[str, Any]:
    value = load_json_bytes(raw, label)
    expected_fields = {
        "schema_version",
        "round_id",
        "capture_id",
        "attestation_run_id",
        "candidate_git_sha",
        "candidate_package_sha256",
        "downstream",
        "workload_id",
        "input_sha256",
        "started_at_unix_ms",
        "finished_at_unix_ms",
        "baseline_samples",
        "candidate_samples",
    }
    if set(value) != expected_fields:
        raise VerificationError(f"{label} must contain the exact round schema fields")
    if value.get("schema_version") != DOWNSTREAM_ROUND_SCHEMA_VERSION:
        raise VerificationError(
            f"{label} schema_version must be {DOWNSTREAM_ROUND_SCHEMA_VERSION}"
        )
    if value.get("round_id") != round_id:
        raise VerificationError(f"{label} round_id does not match its descriptor")
    capture_id = sha256_hex(value.get("capture_id"), f"{label} capture_id")
    if value.get("attestation_run_id") != artifact.run_id:
        raise VerificationError(f"{label} attestation_run_id does not match artifact")
    candidate_sha = validated_git_object(
        value.get("candidate_git_sha"),
        f"{label} candidate_git_sha",
    )
    if candidate_sha.lower() != revision.lower():
        raise VerificationError(f"{label} candidate_git_sha does not match HEAD")
    package_digest = sha256_hex(
        value.get("candidate_package_sha256"),
        f"{label} candidate_package_sha256",
    )
    if package_digest != candidate_package_sha256:
        raise VerificationError(
            f"{label} candidate package digest does not match validated crate"
        )
    downstream = value.get("downstream")
    if downstream != policy["downstream"]:
        raise VerificationError(f"{label} downstream identity does not match policy")
    if value.get("workload_id") != policy["workload_id"]:
        raise VerificationError(f"{label} workload_id does not match policy")
    if value.get("input_sha256") != input_sha256:
        raise VerificationError(f"{label} input digests do not match bound artifacts")

    started_at = bounded_positive_integer(
        value.get("started_at_unix_ms"),
        f"{label} started_at_unix_ms",
    )
    finished_at = bounded_positive_integer(
        value.get("finished_at_unix_ms"),
        f"{label} finished_at_unix_ms",
    )
    if finished_at <= started_at:
        raise VerificationError(f"{label} time window is not increasing")
    if finished_at - started_at < duration_seconds * 1000:
        raise VerificationError(f"{label} time window is shorter than measurement duration")

    baseline_metrics, baseline_samples = metrics_from_round_samples(
        value.get("baseline_samples"),
        sample_count,
        f"{label} baseline_samples",
    )
    candidate_metrics, candidate_samples = metrics_from_round_samples(
        value.get("candidate_samples"),
        sample_count,
        f"{label} candidate_samples",
    )
    semantic_digest = sha256_bytes(
        canonical_json_bytes(
            {
                "baseline_samples": baseline_samples,
                "candidate_samples": candidate_samples,
            }
        )
    )
    return {
        "capture_id": capture_id,
        "started_at_unix_ms": started_at,
        "finished_at_unix_ms": finished_at,
        "semantic_samples_sha256": semantic_digest,
        "metrics": {
            "baseline": baseline_metrics,
            "candidate": candidate_metrics,
        },
    }


def validate_downstream_traffic(
    artifact: VerifiedArtifact,
    manifest: dict[str, Any],
    policy: dict[str, Any],
    revision: str,
    candidate_package_sha256: str,
) -> dict[str, Any]:
    raw = unique_root_file(
        artifact.files,
        DOWNSTREAM_TRAFFIC_FILENAME,
        artifact.name,
    )
    label = f"artifact {artifact.name} {DOWNSTREAM_TRAFFIC_FILENAME}"
    evidence = load_json_bytes(raw, label)
    expected_evidence_fields = {
        "schema_version",
        "policy_id",
        "manifest_id",
        "candidate_git_sha",
        "crate",
        "downstream",
        "workload_id",
        "input_artifacts",
        "crate_feature_matrix",
        "package_matrix",
        "measurement",
    }
    if set(evidence) != expected_evidence_fields:
        raise VerificationError(
            f"{label} must contain the exact downstream evidence schema fields"
        )
    if evidence.get("schema_version") != 2:
        raise VerificationError(f"{label} schema_version must be 2")
    if evidence.get("policy_id") != policy["policy_id"]:
        raise VerificationError(f"{label} policy_id does not match manifest")
    if evidence.get("manifest_id") != manifest.get("manifest_id"):
        raise VerificationError(f"{label} manifest_id does not match")
    candidate_sha = validated_git_object(
        evidence.get("candidate_git_sha"),
        f"{label} candidate_git_sha",
    )
    if candidate_sha.lower() != revision.lower():
        raise VerificationError(f"{label} candidate_git_sha does not match HEAD")

    policy_crate = policy["crate"]
    crate = evidence.get("crate")
    if not isinstance(crate, dict):
        raise VerificationError(f"{label} crate object is missing")
    for field in ("name", "version", "package_filename"):
        if crate.get(field) != policy_crate[field]:
            raise VerificationError(f"{label} crate.{field} does not match policy")
    package_candidate_sha = validated_git_object(
        crate.get("candidate_git_sha"),
        f"{label} crate.candidate_git_sha",
    )
    if package_candidate_sha.lower() != revision.lower():
        raise VerificationError(
            f"{label} crate.candidate_git_sha does not match HEAD"
        )
    package_digest = sha256_hex(
        crate.get("package_sha256"),
        f"{label} crate.package_sha256",
    )
    package_filename = policy_crate["package_filename"]
    package = unique_root_file(artifact.files, package_filename, artifact.name)
    if sha256_bytes(package) != package_digest:
        raise VerificationError(
            f"{label} crate package SHA-256 does not match artifact content"
        )
    if package_digest != candidate_package_sha256:
        raise VerificationError(
            f"{label} crate package does not match the independently validated "
            "candidate package"
        )
    validate_crate_package_vcs(
        package,
        package_filename,
        policy_crate["name"],
        policy_crate["version"],
        revision,
        label,
    )

    policy_downstream = policy["downstream"]
    downstream = evidence.get("downstream")
    if not isinstance(downstream, dict):
        raise VerificationError(f"{label} downstream object is missing")
    if downstream.get("repository") != policy_downstream["repository"]:
        raise VerificationError(
            f"{label} downstream.repository does not match policy"
        )
    source_sha = validated_git_object(
        downstream.get("source_git_sha"),
        f"{label} downstream.source_git_sha",
    )
    if source_sha.lower() != policy_downstream["source_git_sha"].lower():
        raise VerificationError(
            f"{label} downstream.source_git_sha does not match policy"
        )
    if evidence.get("workload_id") != policy["workload_id"]:
        raise VerificationError(f"{label} workload_id does not match policy")

    inputs = evidence.get("input_artifacts")
    if not isinstance(inputs, dict) or set(inputs) != REQUIRED_DOWNSTREAM_INPUTS:
        raise VerificationError(
            f"{label} input_artifacts must contain Cargo.lock, dependency graph, "
            "build config, dataset summary, and package matrix"
        )
    input_provenance: list[dict[str, str]] = []
    input_digest_map: dict[str, str] = {}
    used_filenames = {
        "score-attestation.json",
        DOWNSTREAM_TRAFFIC_FILENAME,
        package_filename,
    }
    for role in sorted(REQUIRED_DOWNSTREAM_INPUTS):
        expected_filename = policy["required_input_artifacts"][role]
        if expected_filename in used_filenames:
            raise VerificationError(
                f"{label} reuses artifact filename {expected_filename}"
            )
        used_filenames.add(expected_filename)
        descriptor = validate_raw_artifact(
            artifact.files,
            inputs[role],
            expected_filename,
            artifact.name,
            f"{label} input_artifacts.{role}",
        )
        input_provenance.append({"role": role, **descriptor})
        input_digest_map[role] = descriptor["sha256"]

    matrix_counts = validate_downstream_package_matrix(evidence, policy, label)
    measurement = evidence.get("measurement")
    if not isinstance(measurement, dict):
        raise VerificationError(f"{label} measurement object is missing")
    for field, policy_field in (
        ("warmup_seconds", "minimum_warmup_seconds"),
        ("duration_seconds", "minimum_duration_seconds"),
    ):
        value = positive_integer(
            measurement.get(field),
            f"{label} measurement.{field}",
        )
        if value < policy["measurement"][policy_field]:
            raise VerificationError(
                f"{label} measurement.{field} is below policy minimum"
            )
    rounds = measurement.get("rounds")
    if not isinstance(rounds, list):
        raise VerificationError(f"{label} measurement.rounds must be an array")
    if len(rounds) < policy["measurement"]["minimum_rounds"]:
        raise VerificationError(
            f"{label} does not contain the minimum independent measurement rounds"
        )
    round_provenance: list[dict[str, Any]] = []
    round_digests: set[str] = set()
    semantic_digests: set[str] = set()
    capture_ids: set[str] = set()
    previous_finished_at: int | None = None
    for index, round_evidence in enumerate(rounds, start=1):
        round_id = f"r{index}"
        round_label = f"{label} measurement round {round_id}"
        if not isinstance(round_evidence, dict):
            raise VerificationError(f"{round_label} must be an object")
        if set(round_evidence) != {"id", "sample_count", "raw_artifact", "metrics"}:
            raise VerificationError(
                f"{round_label} must contain id, sample_count, raw_artifact, and metrics"
            )
        if round_evidence.get("id") != round_id:
            raise VerificationError(
                f"{round_label} id must preserve ordered independent rounds"
            )
        sample_count = positive_integer(
            round_evidence.get("sample_count"),
            f"{round_label} sample_count",
        )
        if sample_count < policy["measurement"]["minimum_samples_per_round"]:
            raise VerificationError(
                f"{round_label} sample_count is below policy minimum"
            )
        round_filename = policy["measurement"][
            "round_artifact_filename_pattern"
        ].format(round=index)
        if round_filename in used_filenames:
            raise VerificationError(
                f"{round_label} reuses artifact filename {round_filename}"
            )
        used_filenames.add(round_filename)
        raw_artifact = validate_raw_artifact(
            artifact.files,
            round_evidence.get("raw_artifact"),
            round_filename,
            artifact.name,
            f"{round_label} raw_artifact",
        )
        if raw_artifact["sha256"] in round_digests:
            raise VerificationError(
                f"{label} measurement rounds reuse identical raw traffic evidence"
            )
        round_digests.add(raw_artifact["sha256"])
        round_raw = unique_root_file(
            artifact.files,
            round_filename,
            artifact.name,
        )
        raw_validation = validate_downstream_round_raw(
            round_raw,
            artifact=artifact,
            policy=policy,
            revision=revision,
            candidate_package_sha256=candidate_package_sha256,
            input_sha256=input_digest_map,
            round_id=round_id,
            sample_count=sample_count,
            duration_seconds=measurement["duration_seconds"],
            label=f"{round_label} raw evidence",
        )
        semantic_digest = raw_validation["semantic_samples_sha256"]
        if semantic_digest in semantic_digests:
            raise VerificationError(
                f"{label} measurement rounds reuse semantically identical samples"
            )
        semantic_digests.add(semantic_digest)
        capture_id = raw_validation["capture_id"]
        if capture_id in capture_ids:
            raise VerificationError(
                f"{label} measurement rounds reuse a capture identity"
            )
        capture_ids.add(capture_id)
        started_at = raw_validation["started_at_unix_ms"]
        finished_at = raw_validation["finished_at_unix_ms"]
        if previous_finished_at is not None and started_at < previous_finished_at:
            raise VerificationError(
                f"{label} measurement round time windows overlap"
            )
        previous_finished_at = finished_at
        validate_reported_round_metrics(
            round_evidence.get("metrics"),
            raw_validation["metrics"],
            round_label,
        )
        ratios = validate_downstream_metrics(
            raw_validation["metrics"],
            policy["measurement"]["metric_thresholds"],
            round_label,
        )
        round_provenance.append(
            {
                "id": round_id,
                "sample_count": sample_count,
                "raw_artifact": raw_artifact,
                "capture_id": capture_id,
                "started_at_unix_ms": started_at,
                "finished_at_unix_ms": finished_at,
                "semantic_samples_sha256": semantic_digest,
                "candidate_to_baseline_ratios": ratios,
            }
        )

    return {
        "filename": DOWNSTREAM_TRAFFIC_FILENAME,
        "sha256": sha256_bytes(raw),
        "policy_id": policy["policy_id"],
        "crate": {
            "name": policy_crate["name"],
            "version": policy_crate["version"],
            "package_filename": package_filename,
            "package_sha256": package_digest,
            "candidate_git_sha": revision,
        },
        "downstream": {
            "repository": policy_downstream["repository"],
            "source_git_sha": source_sha,
        },
        "workload_id": policy["workload_id"],
        "matrix_counts": matrix_counts,
        "input_artifacts": input_provenance,
        "rounds": round_provenance,
    }


def frozen_baseline_sha(manifest: dict[str, Any]) -> str:
    frozen = manifest.get("frozen_baseline")
    if not isinstance(frozen, dict):
        raise VerificationError("manifest frozen_baseline object is missing")
    return validated_git_object(
        frozen.get("git_sha"),
        "manifest frozen_baseline.git_sha",
    )


def comparison_condition_ids(
    conditions: dict[str, dict[str, Any]],
    label: str,
) -> set[str]:
    result = {
        condition_id
        for condition_id, condition in conditions.items()
        if condition["assessment"] == "comparison"
    }
    if result != EXPECTED_COMPARISON_CONDITIONS:
        raise VerificationError(
            f"{label} requires the exact three versioned comparison conditions"
        )
    return result


def validate_comparison_envelope(
    comparison: dict[str, Any],
    manifest: dict[str, Any],
    mode: str,
    revision: str,
    label: str,
) -> str:
    if comparison.get("schema_version") != 2:
        raise VerificationError(f"{label} schema_version must be 2")
    if comparison.get("metadata_compatible") is not True:
        raise VerificationError(f"{label} metadata_compatible must be true")
    if comparison.get("manifest_id") != manifest.get("manifest_id"):
        raise VerificationError(f"{label} manifest_id does not match")
    if comparison.get("mode") != mode:
        raise VerificationError(f"{label} mode must be {mode}")

    head_sha = validated_git_object(
        comparison.get("head_git_sha"),
        f"{label} head_git_sha",
    )
    if head_sha.lower() != revision.lower():
        raise VerificationError(f"{label} head_git_sha does not match HEAD")

    base_sha = validated_git_object(
        comparison.get("base_git_sha"),
        f"{label} base_git_sha",
    )
    if mode == "final" and base_sha.lower() != frozen_baseline_sha(manifest).lower():
        raise VerificationError(f"{label} base_git_sha is not frozen baseline")

    performance_verdict = comparison.get("performance_verdict")
    if performance_verdict not in {"pass", "fail"}:
        raise VerificationError(
            f"{label} performance_verdict must be pass or fail"
        )
    return performance_verdict


def validate_passing_comparison_ledger(
    ledger: dict[str, dict[str, Any]],
    mode: str,
    comparison_ids: set[str],
    label: str,
) -> None:
    for condition_id in comparison_ids:
        expected_status = "passed"
        if mode == "pr" and condition_id == PERFORMANCE_FINAL_GEOMEAN:
            expected_status = "final-comparison-required"
        entry = ledger.get(condition_id)
        if entry is None or entry.get("status") != expected_status:
            raise VerificationError(
                f"{label} pass verdict requires {condition_id} status "
                f"{expected_status}"
            )


def validate_performance_comparisons(
    artifact: VerifiedArtifact,
    manifest: dict[str, Any],
    revision: str,
    comparison_condition_ids: set[str],
) -> list[dict[str, str]]:
    if comparison_condition_ids != EXPECTED_COMPARISON_CONDITIONS:
        raise VerificationError(
            "release verification requires the exact three comparison conditions"
        )
    provenance: list[dict[str, str]] = []
    for filename in ("comparison-r1.json", "comparison-r2.json"):
        raw = unique_root_file(artifact.files, filename, artifact.name)
        label = f"artifact {artifact.name} {filename}"
        comparison = load_json_bytes(raw, label)
        performance_verdict = validate_comparison_envelope(
            comparison,
            manifest,
            "final",
            revision,
            label,
        )
        if performance_verdict != "pass":
            raise VerificationError(
                f"{label} performance_verdict is not pass"
            )
        raw_ledger = comparison.get("score_ledger")
        if not isinstance(raw_ledger, list):
            raise VerificationError(f"{label} score_ledger is missing")
        ledger = require_unique(
            raw_ledger,
            "id",
            f"{label} comparison condition",
        )
        validate_passing_comparison_ledger(
            ledger,
            "final",
            comparison_condition_ids,
            label,
        )
        provenance.append({"name": filename, "sha256": sha256_bytes(raw)})
    return provenance


def validate_required_attestation(
    artifact: VerifiedArtifact,
    requirement: dict[str, Any],
    condition_id: str,
) -> dict[str, Any]:
    expected_workflow = requirement["workflow"]
    actual_workflow = artifact.run_path
    workflow_matches = actual_workflow == expected_workflow
    if not workflow_matches and actual_workflow.startswith(expected_workflow + "@"):
        workflow_ref = actual_workflow[len(expected_workflow) + 1 :]
        workflow_matches = GITHUB_WORKFLOW_REF.fullmatch(workflow_ref) is not None
    if not workflow_matches:
        raise VerificationError(
            f"artifact {artifact.name} run path {actual_workflow} does not match "
            f"{expected_workflow}"
        )
    if artifact.run_event not in requirement["events"]:
        raise VerificationError(
            f"artifact {artifact.name} run event {artifact.run_event} is not allowed"
        )
    claim_identity = (condition_id, requirement["claim"])
    if claim_identity not in artifact.claims:
        raise VerificationError(
            f"artifact {artifact.name} does not attest passed claim "
            f"{condition_id}/{requirement['claim']}"
        )
    return {
        "claim": requirement["claim"],
        "artifact": artifact.name,
        "artifact_id": artifact.artifact_id,
        "artifact_sha256": artifact.digest,
        "workflow": artifact.run_path,
        "run_id": artifact.run_id,
        "event": artifact.run_event,
    }


def score_result(
    result: dict[str, Any],
    ledger: list[dict[str, Any]],
    baseline: int,
    target: int,
    passed: bool,
    scope: str,
) -> dict[str, Any]:
    awarded = sum(entry["awarded"] for entry in ledger)
    total = baseline + awarded
    complete = all(entry["status"] == "passed" for entry in ledger)
    target_met = complete and total >= target
    result["score_ledger"] = ledger
    result["score"] = {
        "baseline": baseline,
        "target": target,
        "awarded": awarded,
        "total": total,
        "complete": complete,
        "target_met": target_met,
    }
    result["gate_scope"] = scope
    result["score_verdict"] = "pass" if target_met else "incomplete"
    result["verdict"] = "pass" if passed else "fail"
    return result


def verify_comparison(
    args: argparse.Namespace,
    manifest: dict[str, Any],
    conditions: dict[str, dict[str, Any]],
    baseline: int,
    target: int,
    revision: str,
) -> tuple[dict[str, Any], bool]:
    comparison_path = getattr(args, "comparison", None)
    if not isinstance(comparison_path, Path):
        raise VerificationError("--comparison is required outside --release-only")
    mode = getattr(args, "mode", None)
    if mode not in {"pr", "final"}:
        raise VerificationError("comparison mode must be pr or final")
    result = load_json(comparison_path)
    performance_verdict = validate_comparison_envelope(
        result,
        manifest,
        mode,
        revision,
        "comparison",
    )
    required_comparison_ids = comparison_condition_ids(
        conditions,
        "comparison verification",
    )

    raw_automatic = result.get("score_ledger")
    if not isinstance(raw_automatic, list):
        raise VerificationError("comparison score_ledger is missing")
    automatic = require_unique(raw_automatic, "id", "comparison condition")
    unknown = set(automatic) - set(conditions)
    if unknown:
        raise VerificationError(
            "comparison score_ledger contains unknown conditions: "
            + ", ".join(sorted(unknown))
        )
    if performance_verdict == "pass":
        validate_passing_comparison_ledger(
            automatic,
            mode,
            required_comparison_ids,
            "comparison",
        )

    ledger: list[dict[str, Any]] = []
    for condition_id, condition in conditions.items():
        status = "external-evidence-required"
        passed_condition = False
        if condition["assessment"] == "comparison":
            automatic_entry = automatic.get(condition_id)
            if automatic_entry is None:
                status = "comparison-assessment-missing"
            else:
                raw_status = automatic_entry.get("status")
                status = (
                    raw_status
                    if isinstance(raw_status, str)
                    else "comparison-assessment-missing"
                )
                passed_condition = status == "passed"
        ledger.append(
            {
                "id": condition_id,
                "dimension": condition["dimension"],
                "points": condition["points"],
                "assessment": condition["assessment"],
                "status": status,
                "awarded": condition["points"] if passed_condition else 0,
            }
        )

    passed = performance_verdict == "pass"
    scope = "pr-performance" if mode == "pr" else "final-performance"
    result = score_result(result, ledger, baseline, target, passed, scope)
    return result, passed


def verify_release(
    args: argparse.Namespace,
    manifest: dict[str, Any],
    conditions: dict[str, dict[str, Any]],
    baseline: int,
    target: int,
    revision: str,
) -> tuple[dict[str, Any], bool]:
    candidate_package_path = getattr(args, "candidate_package", None)
    if not isinstance(candidate_package_path, Path):
        raise VerificationError("--candidate-package is required for --release-only")
    owner, repository, repository_full_name = parse_repository(
        getattr(args, "github_repository", None)
    )
    token = os.environ.get("GITHUB_TOKEN", "").strip()
    if not token:
        raise VerificationError("GITHUB_TOKEN is required for --release-only")
    api_url = os.environ.get("GITHUB_API_URL", "https://api.github.com").strip()
    if not api_url:
        raise VerificationError("GITHUB_API_URL cannot be empty")

    requirements = {
        condition_id: condition_attestations(condition)
        for condition_id, condition in conditions.items()
    }
    required_claims = {
        requirement["claim"]
        for condition_requirements in requirements.values()
        for requirement in condition_requirements
    }
    ci_platform_policy = (
        validate_ci_platform_policy(manifest)
        if CI_PLATFORMS_CLAIM in required_claims
        else None
    )
    safety_stable_policy = (
        validate_safety_stable_policy(manifest)
        if SAFETY_STABLE_CLAIM in required_claims
        else None
    )
    downstream_policy = (
        validate_downstream_policy(manifest)
        if DOWNSTREAM_TRAFFIC_CLAIM in required_claims
        else None
    )
    candidate_package = (
        validate_candidate_package(
            candidate_package_path,
            downstream_policy,
            revision,
        )
        if downstream_policy is not None
        else None
    )
    for condition_id, condition in conditions.items():
        if set(condition.get("required_evidence_kinds", [])) != {
            "tracked-file",
            "verified-ci-attestation",
        }:
            raise VerificationError(
                f"release condition {condition_id} must require tracked-file and "
                "verified-ci-attestation evidence"
            )
    required_comparison_ids = comparison_condition_ids(
        conditions,
        "release verification",
    )

    client = GitHubClient(token, api_url)
    artifact_cache: dict[str, VerifiedArtifact] = {}
    evidence_cache: dict[str, str] = {}
    performance_cache: dict[int, list[dict[str, str]]] = {}
    ci_platform_cache: dict[int, dict[str, Any]] = {}
    safety_stable_cache: dict[int, dict[str, Any]] = {}
    downstream_cache: dict[int, dict[str, Any]] = {}
    ledger: list[dict[str, Any]] = []
    for condition_id, condition in conditions.items():
        tracked = tracked_evidence(condition, revision, evidence_cache)
        attestation_records: list[dict[str, Any]] = []
        for requirement in requirements[condition_id]:
            artifact_name = requirement["artifact"]
            artifact = artifact_cache.get(artifact_name)
            if artifact is None:
                artifact = fetch_artifact(
                    client,
                    owner,
                    repository,
                    repository_full_name,
                    artifact_name,
                    revision,
                )
                artifact_cache[artifact_name] = artifact
            record = validate_required_attestation(
                artifact,
                requirement,
                condition_id,
            )
            if requirement["claim"] == PERFORMANCE_FINAL_CLAIM:
                comparison_provenance = performance_cache.get(artifact.artifact_id)
                if comparison_provenance is None:
                    comparison_provenance = validate_performance_comparisons(
                        artifact,
                        manifest,
                        revision,
                        required_comparison_ids,
                    )
                    performance_cache[artifact.artifact_id] = comparison_provenance
                record["comparisons"] = comparison_provenance
            if requirement["claim"] == CI_PLATFORMS_CLAIM:
                if ci_platform_policy is None:
                    raise VerificationError(
                        "internal CI platform policy resolution failed"
                    )
                platform_provenance = ci_platform_cache.get(artifact.artifact_id)
                if platform_provenance is None:
                    platform_provenance = validate_ci_platforms(
                        artifact,
                        ci_platform_policy,
                        revision,
                    )
                    ci_platform_cache[artifact.artifact_id] = platform_provenance
                record["ci_platforms"] = platform_provenance
            if requirement["claim"] == SAFETY_STABLE_CLAIM:
                if safety_stable_policy is None:
                    raise VerificationError(
                        "internal stable safety policy resolution failed"
                    )
                safety_provenance = safety_stable_cache.get(artifact.artifact_id)
                if safety_provenance is None:
                    safety_provenance = validate_safety_stable(
                        artifact,
                        safety_stable_policy,
                        revision,
                    )
                    safety_stable_cache[artifact.artifact_id] = safety_provenance
                record["safety_stable"] = safety_provenance
            if requirement["claim"] == DOWNSTREAM_TRAFFIC_CLAIM:
                if downstream_policy is None:
                    raise VerificationError(
                        "internal downstream traffic policy resolution failed"
                    )
                downstream_provenance = downstream_cache.get(artifact.artifact_id)
                if downstream_provenance is None:
                    downstream_provenance = validate_downstream_traffic(
                        artifact,
                        manifest,
                        downstream_policy,
                        revision,
                        candidate_package["sha256"],
                    )
                    downstream_cache[artifact.artifact_id] = downstream_provenance
                record["downstream_traffic"] = downstream_provenance
            attestation_records.append(record)

        ledger.append(
            {
                "id": condition_id,
                "dimension": condition["dimension"],
                "points": condition["points"],
                "assessment": condition["assessment"],
                "status": "passed",
                "awarded": condition["points"],
                "tracked_evidence": tracked,
                "required_attestations": attestation_records,
            }
        )

    result: dict[str, Any] = {
        "schema_version": 1,
        "manifest_id": manifest["manifest_id"],
        "mode": "release",
        "candidate_git_sha": revision,
        "github_repository": repository_full_name,
    }
    if candidate_package is not None:
        result["candidate_package"] = candidate_package
    target_met = baseline + sum(condition["points"] for condition in conditions.values()) >= target
    result = score_result(
        result,
        ledger,
        baseline,
        target,
        target_met,
        "release-score",
    )
    return result, result["verdict"] == "pass"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--comparison", type=Path)
    parser.add_argument("--mode", choices=("pr", "final"), default="pr")
    parser.add_argument("--release-only", action="store_true")
    parser.add_argument("--github-repository")
    parser.add_argument("--candidate-package", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.release_only == (args.comparison is not None):
        parser.error("provide exactly one of --release-only or --comparison")
    if args.release_only and not args.github_repository:
        parser.error("--github-repository is required for --release-only")
    if args.release_only and args.candidate_package is None:
        parser.error("--candidate-package is required for --release-only")
    if not args.release_only and args.candidate_package is not None:
        parser.error("--candidate-package is only valid with --release-only")
    return args


def verify(args: argparse.Namespace) -> tuple[dict[str, Any], bool]:
    release_only = bool(getattr(args, "release_only", False))
    comparison = getattr(args, "comparison", None)
    if release_only == (comparison is not None):
        raise VerificationError("provide exactly one of --release-only or --comparison")

    if release_only:
        validate_release_manifest(args.manifest)
    manifest = load_json(args.manifest)
    conditions, baseline, target = manifest_conditions(manifest)
    revision = validated_git_object(
        git_text("rev-parse", "HEAD"),
        "checkout HEAD",
    )
    if release_only:
        return verify_release(
            args,
            manifest,
            conditions,
            baseline,
            target,
            revision,
        )
    return verify_comparison(
        args,
        manifest,
        conditions,
        baseline,
        target,
        revision,
    )


def main() -> int:
    args = parse_args()
    try:
        result, passed = verify(args)
    except VerificationError as exc:
        result = {
            "schema_version": 1,
            "verdict": "fail",
            "score_verdict": "invalid-evidence",
            "errors": [str(exc)],
        }
        passed = False

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(result, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
