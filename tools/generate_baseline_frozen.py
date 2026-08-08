#!/usr/bin/env python3
"""Derive ``tests/fixtures/kagamiqa_baseline_frozen.json`` from a current
migration matrix.

Background (FCEUX11-1.17_计划.md §13.1):
The release-readiness grade (Task 5, ``report/grade.rs``) conservatively
caps at ``C`` unless the runner is invoked with ``--baseline``. With the
frozen baseline pointing at this file, every advisory known-limit failure
becomes ``fail_to_fail`` and the runner outputs the ``B`` grade — exactly
the v1.17 convergence target (C → B). Any *new* advisory FAIL would land
in the ``new_test(FAIL)`` bucket and cap the grade back at ``C``,
producing the precise regression signal the grader is designed for.

Usage::

    python tools/generate_baseline_frozen.py \
        --matrix build/kagamiqa_migration_matrix.json \
        --output tests/fixtures/kagamiqa_baseline_frozen.json
"""
import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--matrix",
        type=Path,
        required=True,
        help="Source MigrationMatrix JSON (e.g. build/kagamiqa_migration_matrix.json)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Destination PreviousRun JSON (matches kagami_qa::report::baseline::PreviousRun)",
    )
    parser.add_argument(
        "--run-id-suffix",
        default="frozen",
        help="Suffix appended to the matrix run_id; distinguishes this snapshot.",
    )
    args = parser.parse_args()

    if not args.matrix.is_file():
        print(f"error: matrix not found: {args.matrix}", file=sys.stderr)
        return 1

    matrix = json.loads(args.matrix.read_text(encoding="utf-8"))
    details = matrix.get("details") or []
    if not details:
        print(f"error: matrix has no details: {args.matrix}", file=sys.stderr)
        return 1

    results = {}
    for d in details:
        tid = d.get("test_id")
        if not tid:
            continue
        results[tid] = bool(d.get("passed"))

    snapshot = {
        "run_id": str(matrix.get("run_id", "unknown")) + "-" + args.run_id_suffix,
        "generated_at": matrix.get("generated_at") or "1970-01-01T00:00:00Z",
        "results": results,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    fails = sum(1 for v in results.values() if not v)
    print(
        f"Wrote {args.output} ({len(results)} entries; "
        f"{fails} known-limit failure(s) preserved as FAIL → grade B when same set recurs)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
