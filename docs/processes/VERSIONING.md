# Versioning

Rivet uses [CalVer](https://calver.org/) for versioning.

## What is CalVer?

CalVer (calendar versioning) is a [SemVer](https://semver.org/)-compatible versioning scheme based on projects' release cycles.

A [long list](https://calver.org/users.html) of projects have been successfully using CalVer for a long time.

## How does it work?

See the [CalVer versioning scheme](https://calver.org/#scheme).

Rivet specifically uses:

```
YYYY.MINOR.MICRO
```

## Motivations for using CalVer

- We don't ship breaking changes, which makes SemVer less effective. Any significant changes we have _must_ have graceful migrations that run automatically.
- We ship on a rolling basis, which makes including the year & release cycle more informational.
- Maintain compatibility with SemVer for releasing patches for older versions.
- Versions like `2023.2.1` are more intuitive than a verbose SemVer version like `1.46.1`.

## Why not date by month/week?

Many projects date their releases by month or week. (e.g. Ubuntu & NixOS use `YYYY.0M`, Tesla uses `YYYY.MM.MICRO`)

**Release schedule**

Internally, Rivet does not have a structured release cycle (yet). We encourage shipping code as fast as possible instead of creating a rigid schedule that all projects have to synchronize to. This lets us deliver features to users faster (often within 48h) and removes the friction around managing multiple projects.

As we scale, this will likely change.

**Minor patches**

When we patch a security issue, we must patch older minor Rivet versions to stay compatible with SemVer. If we ship a minor version too frequently (e.g. on a weekly basis), it becomes difficult to patch older versions.
