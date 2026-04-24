# Gotchas

Every trap encountered during design of this flow. **Read before writing any push command.**

## `gt ls` visual order ≠ PR parent order

**Problem:** Inferring a PR's parent from its visual neighbor in `gt ls` produces wrong commit counts and false "parallel history" findings when lower branches have been force-pushed.

**Why:** Graphite tracks PR parents as `baseRefName` on the GitHub PR. Sometimes that's a real branch name (e.g., `feat/sqlite-vfs-v2`), sometimes a synthetic `graphite-base/<pr#>` ref that pins the original parent SHA at PR-creation time. When the neighboring branch gets rewritten, the `graphite-base/` ref stays pinned to the old snapshot — so the PR's diff is still correct, but the neighbor no longer matches.

**Rule:** Always read `gh pr view <N> --json baseRefName`. Never use the branch visually below a branch in `gt ls`.

## `gt sync` silently skips frozen branches

**Problem:** Running `gt sync` after main moves does not restack frozen branches. If any frozen branch sits between main and the target, the whole chain stays anchored to the pre-move main SHA. The `merge-base --is-ancestor` check will still say NO even though sync just "succeeded".

**Why:** Freeze is opt-out-of-restack. By design. Usually set on branches whose SHAs must not change because they've been reviewed-and-approved in that exact shape.

**Rule:** After `gt sync`, check `git merge-base --is-ancestor origin/main origin/<target>` independently. If NO, enumerate frozen branches in the downstack with `gt ls | grep frozen`, unfreeze each, then `gt restack --downstack`.

## Pushing `target:main` without FF safety deletes commits on main

**Problem:** `git push origin <target>:main` is a non-FF push if target isn't a descendant of main. GitHub rejects non-FF unless forced. Admin-bypass force-push then **replaces** main with target's tip, silently discarding every commit on main that isn't in target's ancestry.

**Symptom user mentioned:** "I've lost commits doing this before."

**Rule:** Before the main push, two checks must both pass:
- `git merge-base --is-ancestor origin/main origin/<target>` returns YES.
- `git rev-list --count origin/<target>..origin/main` returns 0.

If the second is nonzero, there are commits on main not in target. Must be resolved (via restack pulling main into the chain, or via merging main into target) before the FF push is safe.

## `gt squash` out-of-order rewrites the parent tree

**Problem:** If you squash a middle-of-stack branch before its upper neighbors, the upper neighbors' base SHAs shift. Their PR commit counts suddenly balloon (new commits appear to include commits that just left the now-squashed lower branch). You can't reliably re-validate "which branches still need squashing" mid-flow.

**Rule:** Always squash top-down. Start with the branch closest to the tip and work down. Each squash is then independent.

## Stale git worktrees break rebases

**Problem:** Rebase errors like `fatal: 'X' is already checked out at '/tmp/foo'` even when `/tmp/foo` doesn't exist anymore. Git's worktree registration outlives the directory.

**Rule:** Run `git worktree list` at start. Anything marked `prunable` = stale registration. Clean with `git worktree prune -v`. Branches themselves stay; only registration records are removed.

## `Everything up-to-date` during push can hang

**Problem:** `git push ... --force-with-lease ...` sometimes prints "Everything up-to-date" but the command doesn't exit for a long time afterward (server-side hook or post-processing). User may interrupt thinking it's stuck.

**Rule:** After the user interrupts, verify state with `git fetch origin <branch> && git rev-parse origin/<branch>` vs local. If they match, the push actually landed — safe to proceed.

## Ralph-style commits

**Pattern:** `^[a-z]+(\(.+?\))?!?:\s*\[?US-\d+\]?\s*-\s`. Examples:
- `feat: [US-043] - [rivetkit-core: onMigrate lifecycle hook]`
- `feat: US-042 - Schema validation: Zod for user-provided specs`
- `feat: [US-049] - [Inspector: BARE schema definition with vbare versioning]`

**Why flag them:** They're agent-generated task markers, usually in unsquashed form. Indicates a branch that should be squashed before merging so the landed commit message reads cleanly instead of exposing task tracker IDs.

**Not blocking:** Informational only. Present the list, let user decide whether to squash.

## Admin bypass in push output

After the main push, output includes:
```
remote: Bypassed rule violations for refs/heads/main:
remote: - Changes must be made through a pull request.
```

This is expected when using admin override on a protected branch. Not an error.

## `graphite-base/<pr#>` refs get GC'd

**Problem:** Sometimes `git fetch origin graphite-base/<N>` returns `couldn't find remote ref`. Graphite may garbage-collect old graphite-base refs after sync.

**Rule:** If the baseRefName is `graphite-base/<N>` and the fetch fails, re-run `gt submit` (or `gt sync`) to recreate the ref. Alternatively, substitute the PR's current baseRefName lookup fresh from `gh pr view`.

## PRs with non-main bases DO NOT auto-close as MERGED

**Problem:** Older versions of this skill claimed every in-scope PR auto-closes as **MERGED** after the FF push because its head SHA ends up in main's history. That is wrong.

GitHub only marks a PR as **MERGED** when head ⊆ the PR's **base branch** (not the default branch). For stacked PRs, the base is a sibling branch, not main, so the fast auto-merge check never fires. There is a slower background sweep that detects "head reachable from default branch" and closes the PR — but it closes it as **CLOSED** (not MERGED), takes 5+ minutes, and in one observed run (stack of 22 PRs, 2026-04-24) only 6/22 had flipped after 3 minutes.

**Symptom user saw:** `gh pr view <n> --json state` returns `"OPEN"` for every in-scope PR, indefinitely, even though the head SHA is literally `origin/main`.

**Rule:** Close the in-scope PRs explicitly in Phase 8. Only PRs whose `baseRefName` is literally `main` (typically the bottom-of-stack PR) will actually auto-flip to MERGED. Everything else stays OPEN until closed.

## `list_merge_path.sh` stops at `graphite-base/<N>` boundaries

**Problem:** The scoping script walks baseRefName from target down until it hits a base that is `graphite-base/<N>`, then stops — because `graphite-base/<N>` points at a commit SHA, not another branch. But there may be more PRs further down whose heads are ancestors of this one (Graphite's default is to anchor the bottom of a stack at a `graphite-base/` ref, and older PRs in the chain keep their own `graphite-base/` refs).

**Symptom user saw:** after FF-pushing main to target, `gt s` flagged several branches as "empty" because their commits were in main — and their PRs (below the first `graphite-base/` boundary) were still OPEN.

**Rule:** Don't rely on `list_merge_path.sh` to enumerate every PR that will be affected. Phase 8 closes **every open PR whose head is an ancestor of the new main** — that catches the ones below the first `graphite-base/` boundary automatically. Treat `list_merge_path.sh` output as the Phase 6 confirmation preview, not the authoritative "everything that will close" list.

## Local vs remote SHA mismatch after `gt restack`

**Problem:** `gt restack` rewrites branch SHAs locally but does NOT push. Remote still points to old SHAs until explicit `gt submit` or `git push --force-with-lease`. If you re-validate by reading `origin/<branch>`, you'll see the old state and think restack didn't work.

**Rule:** After restack, always check LOCAL SHAs (`git rev-parse <branch>`), not remote (`git rev-parse origin/<branch>`). Only trust remote state after a confirmed push.

## Don't use `--force`, use `--force-with-lease`

`--force` overwrites the remote unconditionally. If someone else pushed to the branch since you fetched, their work is lost.

`--force-with-lease` checks that the remote tip matches what you last fetched. If someone pushed in between, the command fails safely.

## Cascade blast radius when squashing lower in the stack

Squashing a branch cascades a restack through every branch above it. For a squash near the bottom of a 20-branch stack, `gt submit --stack` after will force-push ~20 PR branches. If you only want to merge a subset (say, branches 1-10), consider:
- Do the squash after the merge, not before.
- Or, scope the force-pushes: `git push origin --force-with-lease <b1> <b2> ... <target>` — only the branches in the merge path. Avoid `gt submit --stack` which hits everything.
