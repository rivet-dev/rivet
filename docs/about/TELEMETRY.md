# Telemetry

By default, Rivet automatically makes API requests to a centralized server (currently
[PostHog](https://posthog.com/)) to provide rudimentary information about how Rivet OSS is being used in the
wild.

This document is intended to be as transparent as possible about what we collect and our motivations behind
it.

## Why does Rivet include telemetry?

Rivet collects telemetry for three main reasons:

- **Diagnose issues** Help us diagnose issues users are having on non-standard setups
- **Focus on widely used features** Let us know which services are being used & require more attention
- **Track growth** We invest day and night in to building open source software that anyone can use to
  distribute multiplayer games; tracking the project's growth in the wild helps us get an accurate sense of
  our reach

These metrics are never shared publicly without explicit consent.

## Disabling telemetry

**If you disable telemetry, please let us know why in our [Discord](https://discord.gg/BG2vqsJczH). We work
hard to make sure we respect your privacy & security.**

Add the following to your namespace config:

```toml
[rivet.telemetry]
disable = true
```

## What do we collect?

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

- `player_count` helps us understand how well the system is performing under load on the provided
  configuration & providers

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
