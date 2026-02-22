# RivetKit React Client

Use this skill when building React apps that connect to Rivet Actors with `@rivetkit/react`.

## First Steps

1. Install the React client (latest: {{RIVETKIT_VERSION}})
   ```bash
   npm install @rivetkit/react@{{RIVETKIT_VERSION}}
   ```
2. Create hooks with `createRivetKit()` and connect with `useActor()`.

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
