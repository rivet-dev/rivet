# Frontend Feature Flags

The dashboard runs against several deployment flavors (full cloud, OSS self-host, enterprise/on-prem). A feature flag lets each flavor turn a capability on or off without forking the code. Flags are the mechanism that keeps one frontend build serving every flavor.

## Where flags live

All flags are defined in one place: [frontend/src/lib/features.ts](../../frontend/src/lib/features.ts). Consume them with:

```ts
import { features } from "@/lib/features";

if (features.platform) {
  // cloud-platform-only UI
}
```

- Source of truth at runtime is the `VITE_FEATURE_FLAGS` env var: a comma-separated list of enabled flag names.
- In dev (`import.meta.env.DEV`), a `localStorage` key `FEATURE_FLAGS` overrides the env var so a flavor can be simulated locally.
- **Unset (`undefined`) means every flag is on** — that is the full cloud build. An empty/explicit list opts in only to the named flags.
- Some flags imply others (e.g. `platform` requires `auth`; `acl` is implied by `platform`). Encode those dependencies in `features.ts`, not at each call site.

## Current flags

| Flag | Meaning |
| --- | --- |
| `auth` | Dashboard ships a login/register flow. NOT a proxy for "engine requires credentials." |
| `platform` | Cloud platform stack: publishable-token endpoint, billing, projects, multi-tenancy. Implies `auth`. (`multitenancy` is a legacy alias accepted during rollover.) |
| `acl` | Engine enforces token auth on the public endpoint. Implied by `platform`; set independently for enterprise. |
| `billing` | Billing UI. |
| `captcha` | Turnstile captcha on auth forms. Requires `auth`. |
| `support` | Support/help affordances. |
| `branding` | Rivet branding chrome. |
| `datacenter` | Datacenter-related UI. |
| `danger-zone` | Destructive settings actions (`features.dangerZone`). |

Deployment flavors map to flag sets roughly as: **cloud** = all on; **OSS** = `auth`/`platform`/`acl` off; **enterprise** = `acl` on, `auth`/`platform` off (engine enforces auth without a login UI). Do not treat `platform`/`auth` as "engine requires credentials" — that is `acl`.

## When to add a new flag

**A new pack of features must ship behind a feature flag whenever it is significant or not universally available across deployment flavors.** The goal is that every flavor can freely enable or disable it.

Add a flag when the feature:

- Is unavailable, restricted, or behaves differently on at least one deployment flavor (cloud / OSS / enterprise), OR
- Introduces a substantial new surface (a whole panel, page, settings section, or subsystem) that a flavor may want to turn off.

Do **not** add a flag for:

- Small, universal changes (bug fixes, copy tweaks, layout polish, a single button that every flavor always shows).
- Anything that is always on everywhere — that is just code.

Avoid flag sprawl: do not gate everything. One flag per meaningful capability, not per component.

**If you are unsure whether a feature needs a flag, confirm with the user before adding (or omitting) one.** When in doubt, ask rather than guessing — flags are hard to remove once flavors depend on them.

## Consistency rules

- Add the flag in `features.ts` with a one-line comment on what it gates and any implied dependencies; do not read `VITE_FEATURE_FLAGS` or `import.meta.env` directly elsewhere.
- Name flags after the capability (`billing`, `support`), not the deployment (`enterprise-only`). Deployment flavor is composed from flags, not the other way around.
- Encode flag-implies-flag relationships in `features.ts` so call sites stay simple booleans.
- When a flag changes the set of flavors, update the deployment-flavor mapping above and any docs that describe what each flavor ships.
