# Phase detail

Per-phase procedural detail for the stack-merge flow. SKILL.md has the high-level flow; this file has the specifics each phase.

## Phase 1 — Scope

Input: `<target-branch>` from user.

### Procedure

```bash
./scripts/list_merge_path.sh <target-branch>
```

Walks `gh pr view <b> --json baseRefName` from target down to main. Produces two things:

1. Ordered list of PRs in the merge path (target at top, main-adjacent at bottom).
2. The branch name for each PR (head) + its base.

### Example output

```
PR #4652  04-14-chore_publish_pin_docker_base_image_refs        <- target
PR #4649  04-14-chore_engine_publish_engine_bases_in_ci
PR #4647  04-13-chore_lockfile_lefthook
PR #4645  fix/surface-native-sqlite-kv-errors
... (down to main)
```

### Decision

Confirm with user: "Scope is N PRs ending at `<target>`. Is this the range you want to land?"

## Phase 2 — Sync

```bash
gt sync --no-interactive
```

### What to read from output

- Branches reported as merged upstream → about to be deleted locally.
- Branches reported `(needs restack)` → unresolved restack state, may need `gt restack` or `gt continue`.
- **Frozen branches listed as "Did not restack because it is frozen"** → this is the critical signal for Phase 3.

### After sync

Compare local `main` to `origin/main`. They should match. If main just moved, note the 10+ frontend commits or whatever is new — that's the gap the restack has to close.

## Phase 3 — Unfreeze

### Identify frozen branches in the merge path

```bash
gt ls | grep frozen
```

Cross-reference against the scope from Phase 1. Only branches in scope matter; ignore frozen branches off other sibling chains.

### Why unfreeze

`gt restack --downstack` won't touch a frozen branch. If any frozen branch sits between target and main, restack can't pull main's latest commits into the chain. The frozen chain stays anchored to its old fork point.

### Procedure

Present the full list to user. Require confirmation before running — even though unfreeze is metadata-only, batching it signals intent.

```bash
for b in <branch1> <branch2> ...; do
  gt unfreeze "$b"
done
```

### Verify

```bash
gt ls | grep frozen | wc -l
```

Should be 0 for in-scope branches (siblings can stay frozen).

## Phase 4 — Restack

### Procedure

From the target branch:

```bash
gt checkout <target>
gt restack --downstack
```

### Conflict handling

**Hand off to user.** Graphite pauses on each conflict with a message like:

```
Hit conflict rebasing <branch> onto <parent>.
Resolve conflicts and run `gt continue`.
```

**Do not auto-resolve.** Conflict resolution is judgment. Typical conflict types in this repo:

- **pnpm-lock.yaml** — regenerable. User decides: take main's, then `pnpm install --lockfile-only` to recompute, or take stack's version if the stack added deps.
- **Frontend files** — real content conflicts. User edits manually.
- **Deleted-vs-modified** — branch deleted a file main modified. User decides whether deletion stands.

### Loop

After each `gt continue`, Graphite resumes and may hit the next branch's conflict. Repeat until restack completes.

### Verify restack completion

```bash
git merge-base --is-ancestor origin/main <target> && echo OK || echo STILL DIVERGED
git rev-list --count <target>..origin/main   # should be 0
```

If STILL DIVERGED: main moved during the restack (someone else pushed). `gt sync` again and loop.

## Phase 5 — Validation

```bash
./scripts/validate.sh <target>
```

Output sections:

1. **FF safety** (must pass)
2. **Divergence** (local vs remote per branch — expected to be divergent after restack)
3. **Commit hygiene** (informational: Ralph patterns, unsquashed branches)
4. **Conflict preview** (scratch worktree merge test — should be clean after a clean restack)
5. **Admin check** (current user's repo permissions)

Any hard failure (#1 or #5) → stop, surface to user.

## Phase 6 — Confirmation gate

Show a single structured block:

```
main BEFORE: <sha>
main AFTER:  <sha>
Landing:     N commits
Losing:      0 commits from main
PRs to close as MERGED:
  #4652  04-14-chore_publish_pin_docker_base_image_refs
  #4649  04-14-chore_engine_publish_engine_bases_in_ci
  #4647  04-13-chore_lockfile_lefthook
  ...
Branches to force-push: N (same set)
```

Require explicit "yes" / "approve" / "proceed". Any other response = abort.

## Phase 7 — Execute

```bash
./scripts/exec_merge.sh <target> --confirm
```

The script does two things:

1. Batch force-push all merge-path branches in one git transaction:
   ```bash
   git push origin --force-with-lease <b1> <b2> ... <target>
   ```
2. FF push to main:
   ```bash
   git push origin origin/<target>:main
   ```

### Known quirks

- Push 1 may print "Everything up-to-date" before exiting slowly due to server hooks. If user interrupts, verify with `git rev-parse origin/<branch>` — if it matches local, the push landed. See gotchas.md.
- Push 2 shows `remote: Bypassed rule violations for refs/heads/main` — that's admin override, expected.

## Phase 8 — Verify + cleanup

### Poll PR states

```bash
for pr in <list-from-phase-1>; do
  gh pr view $pr --json state,mergedAt
done
```

Expect every PR in state `MERGED`. GitHub may lag 30-60s on the top 2-3 PRs in the stack; retry once after 20s if needed.

### Clean up locally

```bash
gt sync  # deletes merged branches locally, updates main
```

### Report

- Final main SHA.
- Count of PRs merged.
- Any PRs still OPEN after the grace period (need manual investigation — likely GitHub missed detecting the merge).
- Any upstack branches that now show `(needs restack)` (out of scope for this merge but the user may want to address later).
