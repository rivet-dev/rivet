---
name: stack-merge
description: Bulk-merge a Graphite stack into main via admin-bypass fast-forward push. Replaces the Graphite merge queue for rivet-dev/rivet. Invoke when the user says "merge the stack at <branch>", "ship everything through <branch>", "bulk-merge up to <branch>", "fast-forward main to <branch>", or any request to land multiple stacked PRs at once in one shot. Requires repo admin permissions (bypasses branch protection), Graphite (gt) CLI, and GitHub (gh) CLI. After the FF push, closes all in-scope PRs explicitly with `gh pr close` in parallel — GitHub does not auto-mark them as MERGED because their bases point at sibling stack branches, not main.
---

# Stack Merge

Land an entire Graphite downstack into main in one admin-bypass fast-forward push, then close every in-scope PR explicitly. No per-PR CI gates, no merge commits on main, no `gh pr merge` per PR. PRs end up in **Closed** state (not MERGED) unless their base was already `main`, but their commits are in main's history.

This skill **replaces** Graphite's merge queue — use when you want the outcome Graphite's queue produces (linear history, all PRs merged) but without waiting for the queue. Requires admin bypass on `main` branch protection.

## When NOT to use

- User wants normal single-PR merge → use `gh pr merge`.
- User isn't a repo admin → push will reject (tell them to use Graphite's queue).
- Stack tip has failing CI and user cares → queue would refuse; pause here too.

## Preconditions (verify at start; bail early)

1. **Admin/bypass on main**: `gh api repos/rivet-dev/rivet/branches/main/protection` — check user is in `bypass_pull_request_allowances.users[]` or has repo admin. If not, stop.
2. **Graphite initialized**: `gt ls` succeeds.
3. **Clean working tree**: `git status --short` empty. In-flight rebase/merge state must be resolved first.
4. **Worktree sanity**: `git worktree list` — prune any `prunable` entries with `git worktree prune`. Stale worktrees break rebases with "already checked out at" errors. See references/gotchas.md.

## The flow (8 phases)

Each phase has a **validation gate** or **confirmation gate**. Writes happen only after the user explicitly approves the final commands.

### Phase 1 — Scope

Input: target branch name from user.

- Enumerate the merge path: walk `gh pr view <branch> --json baseRefName` from target down to `main`. Produces the ordered list of all PRs that will be closed in Phase 8.
- **Do not infer parents from `gt ls` visual order** — use each PR's `baseRefName`. See gotchas.md.
- Script: `scripts/list_merge_path.sh <target-branch>`.

### Phase 2 — Sync

- `gt sync --no-interactive`. Pulls latest main, cleans up merged upstream branches.
- Read output for any `(needs restack)` markers in the merge path.

### Phase 3 — Unfreeze

- List frozen branches in the merge path (`gt ls | grep frozen`).
- `gt sync` **silently skips frozen branches**. If any frozen branch sits between target and main, the chain stays anchored to its old main SHA forever. See gotchas.md.
- Unfreeze each one: `gt unfreeze <branch>`. Operation is metadata-only, no rewrites yet. Present full list to user and confirm before running.

### Phase 4 — Restack downstack

- `gt restack --downstack` from the target branch. Cascades rebase from main up through the (now-unfrozen) chain to target.
- Conflicts: **hand off to user** (`gt continue` after resolving each). Do not drive conflict resolution.
- When restack completes, verify: `git merge-base --is-ancestor origin/main <target>` must return YES. If NO, main moved during the restack — `gt sync` again and loop.

### Phase 5 — Validation gate (read-only)

Script: `scripts/validate.sh <target-branch>`. Checks:

1. FF safety: `git merge-base --is-ancestor origin/main origin/<target>` → must be YES.
2. Commits on main not in target: `git rev-list --count origin/<target>..origin/main` → must be 0 (otherwise FF push deletes commits on main).
3. Local vs remote divergence for every branch in the merge path (restack rewrote SHAs; remote needs updating).
4. Commit hygiene (informational, not blocking):
   - Ralph-style commits (pattern: `^[a-z]+(\(.+?\))?!?:\s*\[?US-\d+\]?\s*-\s`). See scripts/detect_ralph.sh.
   - Unsquashed branches (>1 commit against baseRefName).
5. Conflict preview via scratch worktree + `git merge origin/main --no-commit --no-ff`. Shouldn't conflict after a clean restack, but if it does, loop back to Phase 4.

**Gate**: if any check fails, stop and present findings.

### Phase 6 — Confirmation gate

Present to user before any writes:

- main SHA before/after
- number of commits landing
- list of PRs that will be closed in Phase 8 (the merge path)
- list of branches that will be force-pushed

Require explicit "yes". No proceed-on-silence.

### Phase 7 — Push

Two writes, in order:

1. **Batch force-push all merge-path branches**. Each branch's remote head must match its restacked local SHA so the PR's head-SHA-on-GitHub reflects the post-restack commits. This matters only if restack actually rewrote SHAs; with a clean sync and no-op restack it's a harmless no-op.
   ```bash
   git push origin --force-with-lease \
     <branch1> <branch2> ... <target>
   ```
   Single-transaction multi-refspec push is faster than N sequential pushes. Use `--force-with-lease`, never `--force`.
2. **FF push main to target tip**.
   ```bash
   git push origin origin/<target>:main
   ```
   Admin bypass will be noted in push output ("Bypassed rule violations for refs/heads/main").

### Phase 8 — Close PRs + cleanup

1. **Close every open PR whose head is now reachable from main**, in parallel. This catches both the in-scope PRs and anything below the merge path that also landed.
   ```bash
   main_sha=$(git rev-parse origin/main)
   open_prs=$(gh pr list --state open --limit 200 --json number,headRefOid \
     --jq '.[] | "\(.number)\t\(.headRefOid)"')
   close=()
   while IFS=$'\t' read -r n sha; do
     git merge-base --is-ancestor "$sha" "$main_sha" 2>/dev/null && close+=("$n")
   done <<<"$open_prs"
   for n in "${close[@]}"; do
     (gh pr close "$n" --comment "Landed in main via stack-merge fast-forward push. Commits are in main; closing to match." &)
   done
   wait
   ```
   PRs whose base was `main` may have auto-merged by now — `gh pr close` prints `already closed` for those, which is fine.
2. `gt sync` to clean up locally-merged branches.
3. Report final state: main SHA, count of PRs closed, any PRs still OPEN (shouldn't be any).

**Why explicit close**: GitHub only auto-closes a PR as MERGED when head ⊆ the PR's **base** branch. Most stacked PRs have base = sibling branch, not main, so the cheap auto-merge check doesn't fire. GitHub's slower background sweep eventually closes them as CLOSED (not MERGED), but it's unreliable and can take 5+ minutes. Close them ourselves.

## References

- [references/gotchas.md](references/gotchas.md) — every trap from the flow's design session. **Read this before writing any push commands.**
- [references/phases.md](references/phases.md) — per-phase detail, example outputs, decision trees.

## Scripts

- `scripts/list_merge_path.sh <target>` — enumerate in-scope PRs via baseRefName traversal.
- `scripts/validate.sh <target>` — run all Phase 5 read-only checks.
- `scripts/detect_ralph.sh <target>` — flag Ralph-style commits in the merge path.
- `scripts/exec_merge.sh <target> --confirm` — batch force-push + FF push (Phase 7). Requires explicit `--confirm` flag to execute.

## Core rules

- **Never skip the FF safety check.** `git merge-base --is-ancestor origin/main origin/<target>` must be YES before the main push. If commits exist on main that aren't in target, the push deletes them.
- **`gt ls` visual ordering is not PR parent ordering.** Always use `gh pr view <N> --json baseRefName`. `baseRefName` can be either a real branch name or `graphite-base/<pr#>` — both are valid.
- **`gt sync` silently skips frozen branches.** Freeze is opt-out-of-restack. Any frozen branch between target and main breaks the merge path.
- **Use `--force-with-lease` for all force-pushes.** Bails safely if remote moved; `--force` blindly overwrites.
- **Hand off conflicts during `gt restack`.** Do not pick sides. The user resolves, then runs `gt continue`.
- **Close PRs explicitly in Phase 8, don't wait for auto-close.** GitHub only auto-closes as MERGED when head ⊆ base branch, which almost never holds for stacked PRs. See gotchas.md.
- **Phase 8 closes everything reachable from main, not just `list_merge_path.sh` output.** That script stops at `graphite-base/<N>` boundaries and can miss PRs below.
