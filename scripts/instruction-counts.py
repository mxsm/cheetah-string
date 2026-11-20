#!/usr/bin/env python3
"""Compare per-iteration instruction counts between two checkouts.

Builds scripts/icount/scenarios.rs from the head checkout against the crate
in each checkout, runs every scenario under Valgrind's cachegrind with two
iteration counts, and reports the instructions of one iteration:
(count at --high - count at --low) / (--high - --low). Process start-up and
scenario setup cancel out, so the numbers are exact and repeatable.

A scenario regresses when head runs more than --tolerance (relative) and
more than --min-delta (absolute) instructions above base. The script exits
with status 1 on any regression unless --report-only is given.

Usage:
    python scripts/instruction-counts.py --base BASE_DIR --head HEAD_DIR
        [--summary FILE] [--report-only]
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

HARNESS = Path("scripts") / "icount" / "scenarios.rs"
IREFS = re.compile(r"I\s+refs:\s+([\d,]+)")

MANIFEST = """\
[package]
name = "cheetah-icount"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
cheetah-string = {{ path = "{crate}" }}

[profile.release]
codegen-units = 1
debug = false

[workspace]
"""


@dataclass(frozen=True)
class Row:
    scenario: str
    base: float | None
    head: float
    regressed: bool

    @property
    def change(self) -> float | None:
        if self.base is None or self.base == 0:
            return None
        return self.head / self.base - 1.0


def parse_irefs(stderr: str) -> int:
    """Returns the total instruction count from cachegrind's summary."""
    match = IREFS.search(stderr)
    if match is None:
        raise ValueError("cachegrind output has no 'I refs' line:\n" + stderr)
    return int(match.group(1).replace(",", ""))


def per_iteration(low_count: int, high_count: int, low: int, high: int) -> float:
    if high <= low:
        raise ValueError("--high must exceed --low")
    if high_count < low_count:
        raise ValueError(f"instruction count fell from {low_count} to {high_count}")
    return (high_count - low_count) / (high - low)


def compare(
    base: dict[str, float] | None,
    head: dict[str, float],
    tolerance: float,
    min_delta: float,
) -> list[Row]:
    rows = []
    for scenario, head_count in head.items():
        base_count = None if base is None else base.get(scenario)
        regressed = (
            base_count is not None
            and head_count - base_count > min_delta
            and head_count > base_count * (1.0 + tolerance)
        )
        rows.append(Row(scenario, base_count, head_count, regressed))
    return rows


def render(rows: list[Row], tolerance: float, min_delta: float, note: str) -> str:
    lines = [
        "## Instruction counts per iteration",
        "",
        "Valgrind cachegrind, instructions executed by one iteration of each "
        "scenario in `scripts/icount/scenarios.rs`. A scenario regresses when "
        f"head runs more than {tolerance:.0%} and more than {min_delta:g} "
        "instructions above base.",
    ]
    if note:
        lines += ["", note]
    lines += ["", "| Scenario | Base | Head | Change |", "| --- | ---: | ---: | ---: |"]
    for row in rows:
        base = "-" if row.base is None else f"{row.base:.1f}"
        change = "-" if row.change is None else f"{row.change:+.1%}"
        if row.regressed:
            change = f"**{change} regression**"
        lines.append(f"| {row.scenario} | {base} | {row.head:.1f} | {change} |")
    regressions = [row.scenario for row in rows if row.regressed]
    lines.append("")
    if regressions:
        lines.append(f"**{len(regressions)} regression(s):** " + ", ".join(regressions))
    else:
        lines.append("No regressions.")
    return "\n".join(lines) + "\n"


def build(crate: Path, harness: Path, lockfile: Path | None, work: Path) -> Path:
    """Builds the harness against `crate` and returns the binary path."""
    (work / "src").mkdir(parents=True)
    (work / "Cargo.toml").write_text(
        MANIFEST.format(crate=crate.resolve().as_posix()), encoding="utf-8"
    )
    shutil.copyfile(harness, work / "src" / "main.rs")
    if lockfile is not None and lockfile.is_file():
        # Start from head's lockfile so both builds resolve the same versions.
        shutil.copyfile(lockfile, work / "Cargo.lock")
    subprocess.run(
        ["cargo", "build", "--release", "--quiet", "--manifest-path", str(work / "Cargo.toml")],
        check=True,
    )
    suffix = ".exe" if sys.platform == "win32" else ""
    return work / "target" / "release" / f"cheetah-icount{suffix}"


def count(binary: Path, scenario: str, iterations: int) -> int:
    result = subprocess.run(
        [
            "valgrind",
            "--tool=cachegrind",
            "--cache-sim=no",
            "--cachegrind-out-file=/dev/null",
            str(binary),
            scenario,
            str(iterations),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return parse_irefs(result.stderr)


def measure(binary: Path, scenarios: list[str], low: int, high: int) -> dict[str, float]:
    return {
        scenario: per_iteration(
            count(binary, scenario, low), count(binary, scenario, high), low, high
        )
        for scenario in scenarios
    }


def revision(checkout: Path) -> str:
    result = subprocess.run(
        ["git", "-C", str(checkout), "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
    )
    return result.stdout.strip() or checkout.name


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", type=Path, required=True, help="base checkout")
    parser.add_argument("--head", type=Path, required=True, help="head checkout")
    parser.add_argument("--tolerance", type=float, default=0.02)
    parser.add_argument("--min-delta", type=float, default=2.0)
    parser.add_argument("--low", type=int, default=1_000)
    parser.add_argument("--high", type=int, default=11_000)
    parser.add_argument("--summary", type=Path, help="append the Markdown report here")
    parser.add_argument("--report-only", action="store_true", help="never fail")
    args = parser.parse_args(argv)

    harness = args.head / HARNESS
    lockfile = args.head / "Cargo.lock"
    with tempfile.TemporaryDirectory() as temp:
        temp = Path(temp)
        head_binary = build(args.head, harness, lockfile, temp / "head")
        scenarios = subprocess.run(
            [str(head_binary), "--list"], check=True, capture_output=True, text=True
        ).stdout.split()
        note = f"Base `{revision(args.base)}`, head `{revision(args.head)}`."
        try:
            base_binary = build(args.base, harness, lockfile, temp / "base")
        except subprocess.CalledProcessError:
            base_binary = None
            note += " The scenarios do not build against base, so only head is shown."
        head = measure(head_binary, scenarios, args.low, args.high)
        base = None if base_binary is None else measure(base_binary, scenarios, args.low, args.high)

    rows = compare(base, head, args.tolerance, args.min_delta)
    report = render(rows, args.tolerance, args.min_delta, note)
    print(report)
    if args.summary is not None:
        with args.summary.open("a", encoding="utf-8") as summary:
            summary.write(report)
    if any(row.regressed for row in rows) and not args.report_only:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
