# Ralph Agent Instructions

You are an autonomous coding agent working on a software project.

## Your Task

1. Read the PRD at `prd.json` (in the same directory as this file)
2. Read the progress log at `progress.txt` (check Codebase Patterns section first)
3. Check you're on the correct branch from PRD `branchName`. If not, check it out or create from main.
4. Pick the **highest priority** user story where `passes: false`
5. Implement that single user story
6. Run quality checks (e.g., typecheck, lint, test - use whatever your project requires)
7. Update CLAUDE.md files if you discover reusable patterns (see below)
8. If checks pass, commit ALL changes with message: `feat: [Story ID] - [Story Title]`
9. Update the PRD to set `passes: true` for the completed story
10. Append your progress to `progress.txt`

## Progress Report Format

APPEND to progress.txt (never replace, always append):
```
## [Date/Time] - [Story ID]
- What was implemented
- Files changed
- **Learnings for future iterations:**
  - Patterns discovered (e.g., "this codebase uses X for Y")
  - Gotchas encountered (e.g., "don't forget to update Z when changing W")
  - Useful context (e.g., "the evaluation panel is in component X")
---
```

The learnings section is critical - it helps future iterations avoid repeating mistakes and understand the codebase better.

## Consolidate Patterns

If you discover a **reusable pattern** that future iterations should know, add it to the `## Codebase Patterns` section at the TOP of progress.txt (create it if it doesn't exist). This section should consolidate the most important learnings:

```
## Codebase Patterns
- Example: Use `sql<number>` template for aggregations
- Example: Always use `IF NOT EXISTS` for migrations
- Example: Export types from actions.ts for UI components
```

Only add patterns that are **general and reusable**, not story-specific details.

## Update CLAUDE.md Files

Before committing, check if any edited files have learnings worth preserving in nearby CLAUDE.md files:

1. **Identify directories with edited files** - Look at which directories you modified
2. **Check for existing CLAUDE.md** - Look for CLAUDE.md in those directories or parent directories
3. **Add valuable learnings** - If you discovered something future developers/agents should know:
   - API patterns or conventions specific to that module
   - Gotchas or non-obvious requirements
   - Dependencies between files
   - Testing approaches for that area
   - Configuration or environment requirements

**Examples of good CLAUDE.md additions:**
- "When modifying X, also update Y to keep them in sync"
- "This module uses pattern Z for all API calls"
- "Tests require the dev server running on PORT 3000"
- "Field names must match the template exactly"

**Do NOT add:**
- Story-specific implementation details
- Temporary debugging notes
- Information already in progress.txt

Only update CLAUDE.md if you have **genuinely reusable knowledge** that would help future work in that directory.

## Quality Requirements

- ALL commits must pass your project's quality checks (typecheck, lint, test)
- Do NOT commit broken code
- Keep changes focused and minimal
- Follow existing code patterns

## Clean Fixes — Do Not Apply Hacky Patches

**This PRD is a cleanup pass on real bugs. Each fix must be a clean, root-cause
change — not a temporary or tactical patch.** A correct fix is the smallest
change that addresses the actual cause and leaves the surrounding code as good
as or better than you found it. The following are explicitly disallowed unless
the user explicitly asks for them:

- **No band-aids.** If `resolve_pages` holds the wrong lock, fix the lock — do
  not add a comment claiming the lock is "fine". If a tx is non-atomic, fold
  it into one tx or add a real revalidation read; do not add a retry loop.
- **No "tactical" sleeps.** Do not paper over a race with `tokio::time::sleep`
  or `setTimeout`. If a wakeup is lost, pre-arm the `Notified`. If a counter
  needs an event, pair it with a `Notify` / `watch`. CLAUDE.md (`Performance`
  section) is the binding floor.
- **No retry-until-success in tests.** `vi.waitFor` is acceptable only with a
  preceding `//` line that justifies polling vs awaiting. Retry loops that
  swallow startup or stale-handle races are flake-masking and must be
  replaced with deterministic ordering.
- **No `#[allow(...)]` to silence a warning.** If clippy or rustc warns,
  address the underlying issue. Adding `#[allow(dead_code)]` to keep a dead
  helper alive is the wrong direction — the dead helper should go.
- **No "I'll fix this in a follow-up" comments.** If you discover a related
  bug while implementing a story, either include the fix in scope or write a
  new issue entry — do not leave a `TODO: this is broken` in the code.
- **No new `unwrap()` / `expect("…")` on recoverable paths.** Return
  `anyhow::Error` with `.context(...)` or surface a typed `RivetError`.
- **No copy-paste tests.** If you add a regression test, write the smallest
  case that fails today and passes after the fix. Do not duplicate an
  existing test "with one tweak" — share helpers via `tests/common/`.
- **No "for now" abstractions.** Do not introduce a feature flag, shim, or
  toggle to leave the old behavior reachable. The fix replaces the bug.

**TDD process per story:** (1) write or identify a failing test that exercises
the bug; (2) implement the clean fix; (3) confirm the same test now passes.
Record the failing command in the story `notes` field as you go. If the fix
sketch from `~/.agents/notes/sqlite-review-issues.md` proves wrong on closer
inspection, update the issue file and the story `notes`, then implement the
correct fix — do not silently divergeo from what's tracked.

**When in doubt, ask the user.** A clean fix that takes a few extra minutes to
get right is always preferable to a fast patch that introduces follow-up
debt.

## Browser Testing (If Available)

For any story that changes UI, verify it works in the browser if you have browser testing tools configured (e.g., via MCP):

1. Navigate to the relevant page
2. Verify the UI changes work as expected
3. Take a screenshot if helpful for the progress log

If no browser tools are available, note in your progress report that manual browser verification is needed.

## Stop Condition

After completing a user story, check if ALL stories have `passes: true`.

If ALL stories are complete and passing, reply with:
<promise>COMPLETE</promise>

If there are still stories with `passes: false`, end your response normally (another iteration will pick up the next story).

## Important

- Work on ONE story per iteration
- Commit frequently
- Keep CI green
- Read the Codebase Patterns section in progress.txt before starting
