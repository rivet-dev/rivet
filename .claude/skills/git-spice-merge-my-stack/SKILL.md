---
name: git-spice-merge-my-stack
description: Merge a stacked git-spice branch chain with gh by retargeting each PR to main and merging bottom to top, including conflict recovery via rebase.
license: MIT
compatibility: Requires GitHub CLI (gh), git, and push access.
metadata:
  author: rivet
  version: "1.0"
---

Merge a stacked PR chain.

**Input**: A target branch in the stack (usually the top branch to merge through).

**Goal**: Merge all PRs from the bottom of that stack up to the target branch.

## Steps

1. **Resolve the target PR**
   - Find PR for the provided branch:
     - `gh pr list --state open --head "<target-branch>" --json number,headRefName,baseRefName,url`
   - If no open PR exists, stop and report.

2. **Build the stack chain down to main**
   - Start at target PR.
   - Repeatedly find the PR whose `headRefName` equals the current PR `baseRefName`.
   - Continue until base is `main` or no parent PR exists.
   - If chain is ambiguous, stop and ask the user which branch to follow.

3. **Determine merge order**
   - Merge from **bottom to top**.
   - Example: `[bottom, ..., target]`.

4. **For each PR in order**
   - Retarget to `main` before merge:
     - `gh pr edit <pr-number> --base main`
   - Merge with repository-compatible strategy:
     - Try `gh pr merge <pr-number> --squash --delete-branch=false`
   - If merge fails due conflicts:
     - `gh pr checkout <pr-number>`
     - `git fetch origin main`
     - `git rebase origin/main`
     - Resolve conflicts. If replaying already-upstream commits from lower stack layers, prefer `git rebase --skip`.
     - Continue with `GIT_EDITOR=true git rebase --continue` when needed.
     - `git push --force-with-lease origin <head-branch>`
     - Retry `gh pr merge ... --squash`.

5. **Verify completion**
   - Confirm each PR in chain is merged:
     - `gh pr view <pr-number> --json state,mergedAt,url`
   - Report final ordered merge list with PR numbers and timestamps.

## Guardrails

- Always merge in bottom-to-top order.
- Do not use merge commits if the repo disallows them.
- Do not delete remote branches unless explicitly requested.
- If a conflict cannot be safely resolved, stop and ask the user.
- If force-push is required, use `--force-with-lease`, never `--force`.
- After finishing, return to the user's original branch unless they asked otherwise.
