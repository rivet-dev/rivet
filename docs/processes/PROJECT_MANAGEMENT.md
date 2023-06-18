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
