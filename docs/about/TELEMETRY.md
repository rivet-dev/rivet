# Telemetry

By default, Rivet automatically makes API requests to a centralized server (currently [PostHog](https://posthog.com/)) to provide rudimentary information about how Rivet OSS is being used in the wild.

This document is intended to be as transparent as possible about what we collect and our motivations behind it.

## Why does Rivet include telemetry?

Rivet collects telemetry for three main reasons:

-   **Adjust product investments** Let us know which products are being used & require more attention
-   **Diagnose issues** Help us diagnose issues users are having on non-standard setups
-   **Fundraising** Rough usage metrics help us with fundraising to demonstrate real-world usage

These metrics are never shared publicly without explicit consent.

## What do we collect?

### Bolt

[Source code](/lib/bolt/core/src/utils/telemetry.rs)

**Global**

-   `git_remotes` helps us understand what fork is being used
-   `git_rev` helps us understand what version is being used
-   `os_release` & `uname` help us diagnose issues caused by the host OS
-   `services` help us understand what functionality is being added when modifying Rivet and what we need to focus on improving

```typescript
{
	"$set": {
		"cluster_id": string,
		"ns_id": string,
		"ns_config": object,
		"bolt": {
			"git_remotes": string[],
			"git_rev": string,
			"os_release": Map<string, string>,
			"uname": string,
			"services": Map<string, {}>,
		}
	}
}
```

**bolt_config_generate**

Sent when running `bolt config generate` and on `bolt init`.

```typescript
{
	"ns_id": string,
}
```

**bolt_terraform_apply**

Sent when running `bolt init`, `bolt infra up`, or `bolt terraform apply`.

```typescript
{
	"plan_id": string,
}
```

**bolt_terraform_destroy**

Sent when running `bolt infra destroy` or `bolt terraform destroy`.

```typescript
{
	"plan_id": string,
}
```

**bolt_salt_apply**

Sent when running `bolt init`, `bolt infra up`, or `bolt salt apply`.

```typescript
{
	"filter": string,
	"sls": string[],
}
```

**bolt_up**

Sent when running `bolt init`, `bolt infra up`, or `bolt up`.

```typescript
{
}
```

### Beacon

[Source code](/svc/pkg/telemetry/standalone/beacon/src/lib.rs)

This data is sent once per day.

**Cluster**

Helps us understand how many Rivet clusters are running in the wild.

```typescript
{
	"$set": {
		"ns_id": string,
		"cluster_id": string,
	}
}
```

**Development teams**

Helps us understand the size of game studio we should be investing resources in to.

```typescript
{
	"$set": {
		"ns_id": string,
		"cluster_id": string,
		"team_id": string,
		"display_name": string,
		"create_ts": number,
		"member_count": number,
	}
}
```

**Games**

Helps us understand if developers are running multiple games on a single Rivet cluster.

```typescript
{
	"$set": {
		"ns_id": string,
		"cluster_id": string,
		"game_id": string,
		"name_id": string,
		"display_name": string,
		"create_ts": number,
		"url": string,
	}
}
```

**Game namespaces**

-   `total_users` & `linked_users` helps us understand if Rivet Social is being used
-   `player_count` helps us understand how well the system is scaling in comparison to the player count

```typescript
{
	"$set": {
		"ns_id": string,
		"cluster_id": string,
		"namespace_id": ns_id,
		"name_id": ns.name_id,
		"display_name": ns.display_name,
		"create_ts": ns.create_ts,
		"version": version,
	},
	"total_users": number,
	"linked_users": number,
	"player_count": player_count,
}
```

## Disabling telemetry

**If you disable telemetry, please let us know why in our [Discord](https://discord.gg/BG2vqsJczH). We work hard to make sure we respect your privacy & security.**

Add the following to your namespace config:

```toml
[rivet.telemetry]
disable = true
```

Then run `bolt up telemetry-beacon` to disable the beacon.
