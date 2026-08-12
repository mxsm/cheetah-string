#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path
from typing import Any


PREFIX = "SHARED_BACKING_EVIDENCE="


class EvidenceError(ValueError):
    """Raised when allocation evidence does not satisfy the stable contract."""


def parse_log(log: str) -> dict[str, Any]:
    records = [line[len(PREFIX) :] for line in log.splitlines() if line.startswith(PREFIX)]
    if len(records) != 1:
        raise EvidenceError(f"expected exactly one allocation evidence record, found {len(records)}")

    try:
        value = json.loads(records[0])
    except json.JSONDecodeError as error:
        raise EvidenceError(f"allocation evidence is not valid JSON: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("allocation evidence must be a JSON object")
    return value


def read_log(path: Path) -> str:
    data = path.read_bytes()
    if data.startswith((b"\xff\xfe", b"\xfe\xff")):
        return data.decode("utf-16")
    return data.decode("utf-8-sig")


def require_mapping(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{label} must be an object")
    return value


def require_allocation(
    allocations: dict[str, Any], name: str, expected_count: int, zero_bytes: bool = False
) -> None:
    entry = require_mapping(allocations.get(name), f"allocations.CheetahString.{name}")
    count = entry.get("count")
    allocated_bytes = entry.get("bytes")
    if type(count) is not int or count != expected_count:
        raise EvidenceError(
            f"allocations.CheetahString.{name}.count must be {expected_count}, found {count!r}"
        )
    if type(allocated_bytes) is not int or allocated_bytes < 0:
        raise EvidenceError(f"allocations.CheetahString.{name}.bytes must be a non-negative integer")
    if zero_bytes and allocated_bytes != 0:
        raise EvidenceError(f"allocations.CheetahString.{name}.bytes must be zero")
    if expected_count > 0 and allocated_bytes == 0:
        raise EvidenceError(f"allocations.CheetahString.{name}.bytes must record allocated storage")


def validate_evidence(evidence: dict[str, Any]) -> None:
    if evidence.get("schema_version") != 2:
        raise EvidenceError("schema_version must be 2")

    sizes = require_mapping(evidence.get("object_sizes"), "object_sizes")
    if sizes.get("CheetahString") != 32:
        raise EvidenceError("object_sizes.CheetahString must remain 32 on the 64-bit gate runner")

    allocation_groups = require_mapping(evidence.get("allocations"), "allocations")
    allocations = require_mapping(allocation_groups.get("CheetahString"), "allocations.CheetahString")
    require_allocation(allocations, "borrowed", 1)
    require_allocation(allocations, "owned_exact", 1)
    require_allocation(allocations, "owned_spare", 2)
    for name in ("from_arc_str", "clone", "char", "short_concat"):
        require_allocation(allocations, name, 0, zero_bytes=True)

    invariants = require_mapping(evidence.get("invariants"), "invariants")
    if invariants.get("from_arc_str_pointer_reused") is not True:
        raise EvidenceError("invariants.from_arc_str_pointer_reused must be true")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("log", type=Path, help="captured shared_backing benchmark output")
    args = parser.parse_args()

    try:
        evidence = parse_log(read_log(args.log))
        validate_evidence(evidence)
    except (OSError, EvidenceError) as error:
        print(f"allocation evidence rejected: {error}", file=sys.stderr)
        return 1

    print("allocation evidence verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
