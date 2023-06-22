# Telemetry

Rivet automatically makes API requests to a centralized server to provide information on how Rivet OSS is being used in the wild.

## Why does Rivet include telemetry?

Telemetry is our way of letting us know how Rivet is used in the wild without being in direct contact with developers.

We work hard to make sure Rivet OSS is easy to run and accessible to everyone. We fully expect there to be users that never pay us a dime nor contact our support, so our telemetry system lets us understand how these users use our product.

## What do we track?

### Bolt
 
[Source code](/lib/bolt/core/src/utils/telemetry.rs)

**Global**

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
			"services": Map<string, {}>,
			"uname": string,
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
{}
```

### Beacon

[Source code](/svc/pkg/telemetry/standalone/beacon/src/lib.rs)

This data is sent once per day.

**Cluster**

```typescript
{
	"$set": {
		"ns_id": string,
		"cluster_id": string,
	}
}
```

**Development teams**

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

