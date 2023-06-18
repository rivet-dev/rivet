# Project Management

## Goals of good project management

- Maximize velocity
- Minimize bureaucracy by building better processes & letting individuals do more
- Maximize motivation
- Provide concrete goals & milestones to align the company

## Goals

All work should be categorized under the following goals with the rough distribution of work:

> TODO

## Writing projects

Each project specification should answer:

-   Why
-   What
-   How

https://linear.app/method/introduction#write-project-specs

## Writing issues

Scope issues to be as small as possible

## How changes are made

1. Create a ticket for absolutely any change
2. Create a branch named after the ticket (see "copy git branch name" in Linear)
3. Create a PR
    - Make sure to link any other issues by [adding `Fixes ABC-123`](https://linear.app/docs/github#link-prs)
4. Push your changes
5. Request review
6. PR gets merged
7. See _Deploy process_ for how code gets sent to production

## Pull requests

### Pull request goals

Aim for:

- **Small as possible** Break down your projects in to tiny steps and make a PR for each step
- **48h time-to-PR** Aim to create a pull request within 48h of creating a new branch
- **Minimize related issues** If you find yourself linking more than 3 issues to a single PR, it's probably too big

Following these goals will lead to:

- **Better conversation** Having less code changes provides the opportunity to hold more pointed discussions about the proposed changes
- **Lower chance of missing bugs** Big PRs make it really hard to spot bugs
- **Lower chance of conflict** The longer a branch is deviated from `main`, the higher probability that someone will build on code that you're changing. This is often more complicated to resolve than just a merge conflict.

### Incomplete functionality & feature flagging

It's OK if your PR doesn't actually expose new functionality or exposes incomplete functionality. What matters it that we're making well planned, small changes.

If your feature is a work in progress, put it behind a feature flag by adding a configuration variable in `lib/bolt/config/src/ns.rs`.

### Never create PR without a related issue.

We use Linear internally to track issues, so each PR must be attached to an issue.

### PR dependencies

If you're waiting for a PR to get merged, you can create another fork & PR that depends on the original PR.

Just mention the following in the PR body:

```
Depends on #42
```

## Project scope & grouping issues

-   **1-3 week projects** Keep projects small and approachable
- **1 project = 1 changelog bullet point** Aim to scope projects big enough that they take 1 bullet point in the changelog. Anything more and the project is probably too big.
    -   e.g. image uploads, improved stability
-   **Avoid grouping by time** Creating projects around time windows is an anti-pattern, this is what cycles is for

## Tickets for backlog projects

It's common for a single ticket to spec a large project that will be started in the future. This is OK.

When the project is started, the ticket will cancelled and converted in to a project.

Do not assign estimates to these issues.

## Working on tickets not in projects

> TODO: When do we encourage working on tickets not related to projects?

## Ticket estimates

Tickets are ranked by exponential estimate sizes.

The following should give a good idea of the scale:

- **1 point** A small change like a config tweak or typo
- **2 points** A straightforward task like writing a simple operation or fixing a test
- **4 points** An in-depth task that requires deep focus but achievable in under half a day
- **8 point** A 1-2 day task, often will require subtasks
- **16 points** This should rarely be used and should be broken down in to smaller tasks if possible.

## Refactoring

In an ideal world, systems are designed well and tested thoroughly before merging. But life happens, and refactoring is a reality we have to deal with.

### Hard parts of refactoring

- Merge conflicts with other team members
- Re-educating team members
- Breaking changes

### Tips for refactoring

- **Automation** If making changes that will affect other team members, try writing a script to perform the refactor in case new code gets written by other members that needs to be refactored
- **Documentation** Write a document on what changed in the refactor and mention team members

## Changelog

Keep CHANGELOG.md up to date. This will be used to publish our weekly updates.


**Why not use tickets & PRs as a changelog?**

Tickets and PRs are great internally for people who know the Rivet codebase inside and out. Changelogs are where these changes are communicated clearly to the public who doesn't contribute to Rivet.

**Resources**

- [Startups Write Changelogs](https://medium.com/linear-app/startups-write-changelogs-c6a1d2ff4820) elaborates on some of the motivation behind changelogs
- [Keep a changelog](https://keepachangelog.com/en/1.0.0/) is used as our changelog specification

## Organizing teams

Organize teams around the type of work they will work on. Sometimes this is specific to the product, but it's often broader than a specific product. Teams may overlap sometimes.

## Use other teams as a knowledge resource, not a labor resource

**It's expected tha teams can work on code published by other teams whenever possible.** Asking other teams to do work for you causes an exponential bottleneck: they have more work to do, your work is stalled, and someone else is probably creating more work for you. 

This is a two way street:

- The original team must **document and write thorough tests** for their services so other people can understand it
- The other team must put in the effort to **understand the tests and ask good questions**

For example: the services team works on the matchmaker and the social team works on the party system. Both integrate with each other. It's the job of the services team to document the mechanics of the matchmaker and it's the job of the party team to understand the mechanics.

If the party team gets stuck, ask questions to the services team, don't expect them to do work for you.

Sometimes, code is specialized enough that not everyone is equip to work on it, but this happens the minority of the time.

## Deploy process

When merging code:

1. Pull request gets merged
2. Code gets deployed to `staging` & issues convert to _Validating_

When publishing to production:

1. All _Validating_ issues are manually validated on staging
2. Publish deploy
3. Check all _Validating_ issues again and set to _Complete_
    - If there is a problem, complete the issue and create a new issue

## How to choose what to prioritize

Three options:

-   Bug fixes
-   Enhancements
-   Underlying technical coolness

> TODO

## Choosing what to work on

When choosing to work on something, always make sure the answer is yes to this: is this the highest leverage/impact thing I can work on to reach a given milestone?

## Blocking issues

Mark issues as blockers as often as possible to help prioritize issues

## Cancelling issues

Always document why you cancel an issue

## Documenting milestones

> TODO

## Issues

Each issue needs at least two categories:

-   Enhancement, bug, research
-   Product vertical

Attach the company that has expressed interest in an issue.

**Public GitHub issues**

To publish to GitHub, attach the `Public` tag.

**First good issue**

If this is an easy issue, mark as `first good issue`.

## Triage

> TODO

## Handling conflicting opinions

> TODO
