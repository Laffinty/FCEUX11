#!/usr/bin/env python3
"""Release gate: CI, F11QA and Pages must be green for the commit being released.

Called by .github/workflows/release.yml, but runnable by hand for debugging:

    GITHUB_TOKEN=... python3 scripts/check_release_gates.py \
        --repo owner/name --sha <commit> --timeout-min 90

Why this is a script and not shell in the workflow: the gate needs polling,
JSON handling and a git subprocess, and every heredoc-based variant inside a
YAML block scalar breaks the document (block scalars require indentation, and
a heredoc terminator must start at column zero).

Gate semantics
--------------
* CI and F11QA: the latest run of that workflow for this exact SHA must be
  completed and successful.
* Deploy Pages: pages.yml only runs when web/** changes, so a release commit
  that does not touch web/ has no Pages run of its own. That is not a failure:
  the gate then requires (a) the most recent successful Pages run, and (b) that
  `git diff <that-run's-sha> <this-sha> -- web/` is empty, i.e. what is
  deployed is byte-identical to what is being released.

Exit codes: 0 all green; 1 a gate is red or the wait timed out.
"""

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request

CI_WORKFLOW = "CI"
F11QA_WORKFLOW = "F11QA"
PAGES_WORKFLOW = "Deploy Pages"

TERMINAL_BAD = {"failure", "cancelled", "timed_out", "action_required", "startup_failure"}


def api_get(url, token):
    req = urllib.request.Request(
        url,
        headers={
            "Authorization": "Bearer %s" % token,
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "fceux11-release-gate",
        },
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def latest_run_for_sha(repo, sha, name, token):
    data = api_get(
        "https://api.github.com/repos/%s/actions/runs?head_sha=%s&per_page=100" % (repo, sha),
        token,
    )
    runs = [r for r in data.get("workflow_runs", []) if r.get("name") == name]
    if not runs:
        return None
    runs.sort(key=lambda r: r.get("created_at", ""), reverse=True)
    return runs[0]


def verdict(run):
    if run is None:
        return "absent"
    if run.get("status") != "completed":
        return "pending"
    return "ok" if run.get("conclusion") == "success" else "failed:%s" % run.get("conclusion")


def latest_successful_pages_run(repo, token):
    data = api_get(
        "https://api.github.com/repos/%s/actions/workflows/pages.yml/runs?branch=main&status=success&per_page=1"
        % repo,
        token,
    )
    runs = data.get("workflow_runs", [])
    return runs[0] if runs else None


def web_dir_unchanged(deployed_sha, sha, web_dir):
    """True when web/ is identical between the deployed commit and this one."""
    try:
        proc = subprocess.run(
            ["git", "diff", "--quiet", deployed_sha, sha, "--", web_dir],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
    except FileNotFoundError:
        return False
    return proc.returncode == 0


def pages_verdict(repo, sha, token, page_run, web_dir):
    v = verdict(page_run)
    if v in ("ok", "pending"):
        return "pages-%s" % v
    if v.startswith("failed"):
        return "pages-failed"

    # No Pages run for this commit: fall back to "what is live is what we ship".
    deployed = latest_successful_pages_run(repo, token)
    if deployed is None:
        return "pages-failed"          # never deployed successfully
    deployed_sha = deployed.get("head_sha", "")
    if not deployed_sha:
        return "pages-failed"
    if web_dir_unchanged(deployed_sha, sha, web_dir):
        return "pages-ok-via-diff"
    return "pages-stale"               # web/ changed since the last deploy


def main(argv):
    ap = argparse.ArgumentParser(description="CI/F11QA/Pages gate for a release")
    ap.add_argument("--repo", required=True, help="owner/name")
    ap.add_argument("--sha", required=True, help="commit being released")
    ap.add_argument("--timeout-min", type=int, default=90)
    ap.add_argument("--interval-sec", type=int, default=60)
    ap.add_argument("--web-dir", default="web")
    ap.add_argument("--once", action="store_true", help="single check, no polling")
    args = ap.parse_args(argv)

    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if not token:
        print("ERROR: GITHUB_TOKEN (or GH_TOKEN) is not set", file=sys.stderr)
        return 2

    deadline = time.time() + args.timeout_min * 60
    attempt = 0

    while True:
        attempt += 1
        ci = verdict(latest_run_for_sha(args.repo, args.sha, CI_WORKFLOW, token))
        f11 = verdict(latest_run_for_sha(args.repo, args.sha, F11QA_WORKFLOW, token))
        page_run = latest_run_for_sha(args.repo, args.sha, PAGES_WORKFLOW, token)
        pages = pages_verdict(args.repo, args.sha, token, page_run, args.web_dir)

        print("[%d] CI=%s F11QA=%s Pages=%s" % (attempt, ci, f11, pages), flush=True)

        pages_ok = pages in ("pages-ok", "pages-ok-via-diff")
        if ci == "ok" and f11 == "ok" and pages_ok:
            print("All gates green.")
            return 0

        bad = (
            ci.startswith("failed")
            or f11.startswith("failed")
            or pages in ("pages-failed", "pages-stale")
        )
        if bad:
            print(
                "ERROR: a required gate is red (CI=%s F11QA=%s Pages=%s)" % (ci, f11, pages),
                file=sys.stderr,
            )
            return 1

        if args.once:
            print("Not all gates green yet (single check requested).", file=sys.stderr)
            return 1
        if time.time() >= deadline:
            print(
                "ERROR: timed out after %d min (CI=%s F11QA=%s Pages=%s)"
                % (args.timeout_min, ci, f11, pages),
                file=sys.stderr,
            )
            return 1
        time.sleep(args.interval_sec)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
