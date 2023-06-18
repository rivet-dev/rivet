# Pull requests

## Pull request goals

Aim for:

- **Small as possible** Break down your projects in to tiny steps and make a PR for each step
- **48h time-to-PR** Aim to create a pull request within 48h of creating a new branch
- **Minimize related issues** If you find yourself linking more than 3 issues to a single PR, it's probably too big

Following these goals will lead to:

- **Better conversation** Having less code changes provides the opportunity to hold more pointed discussions about the proposed changes
- **Lower chance of missing bugs** Big PRs make it really hard to spot bugs
- **Lower chance of conflict** The longer a branch is deviated from `main`, the higher probability that someone will build on code that you're changing. This is often more complicated to resolve than just a merge conflict.

## Incomplete functionality & feature flagging

It's OK if your PR doesn't actually expose new functionality or exposes incomplete functionality. What matters it that we're making well planned, small changes.

If your feature is a work in progress, put it behind a feature flag by adding a configuration variable in `lib/bolt/config/src/ns.rs`.

## Never create PR without a related issue.

We use Linear internally to track issues, so each PR must be attached to an issue.

## PR dependencies

If you're waiting for a PR to get merged, you can create another fork & PR that depends on the original PR.

Just mention the following in the PR body:

```
Depends on #42
```