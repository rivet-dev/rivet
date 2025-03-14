# Demo Docker Run Script

This demo runs a Docker container that runs an arbitrary user script. Your application must figure out where to host & download the script.

## Overview

- `rivet.json` Config for what to deploy
- `Dockerfile`
- `src/mian.ts` Example server that will spawn a user script
- `fixtures/*.ts` Example user-provided scripts (your service will download these over HTTP)
- `tests/client.ts` Example script that will create & destroy actors

## Testing

```
docker build -t demo .
docker run -p 8080:8080 -e PORT_HTTP=8080 -e USER_CODE_FILE_NAME=date.ts demo
curl localhost:8080
```

## Deploying

Deploy with:

```
rivet deploy
```

Test with:

```
# `rivet shell` provides credentials to the create_actor.ts script
rivet shell -e "deno run -A scripts/create_actor.ts"
```

To integrate the API on your own server:

```typescript
const { actor } = await client.actor.create({
    project,
    environment,
    body: {
        // This may be whatever you like
        tags: { name: "app", foo: "bar" },
        // Must match the name in rivet.json
        buildTags: { name: "app" },
        region: region.id,
        runtime: {
            environment: {
                // Configure your container like this
                USER_CODE_FILE_NAME: "date.ts",
            },
        },
        network: {
            ports: {
                http: { protocol: "https" }
            }
        },
        // IMPORTANT: Current hardware only provides 1 CPU : 2 GB of RAM ratios. e.g. 0.5 CPU = 1 GB RAM, 2 CPU = 4 GB RAM, etc
        resources: {
            cpu: 1000,
            memory: 2048,
        }
    }
});
```

