# RivetKit JavaScript Client

Use this skill when building JavaScript clients (browser, Node.js, or Bun) that connect to Rivet Actors with `rivetkit/client`.

## First Steps

1. Install the client (latest: {{RIVETKIT_VERSION}})
   ```bash
   npm install rivetkit@{{RIVETKIT_VERSION}}
   ```
2. Create a client with `createClient()` and call actor actions.

## Error Handling Policy

- Prefer fail-fast behavior by default.
- Avoid `try/catch` unless absolutely needed.
- If a `catch` is used, handle the error explicitly, at minimum by logging it.

<!-- CONTENT -->

## Need More Than the Client?

If you need more about Rivet Actors, registries, or server-side RivetKit, add the main skill:

```bash
npx skills add rivet-dev/skills
```

Then use the `rivetkit` skill for backend guidance.
