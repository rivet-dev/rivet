# RivetKit Swift Client

Use this skill when building Swift clients that connect to Rivet Actors with `RivetKitClient`.

## Version

RivetKit version: {{RIVETKIT_VERSION}}

## Error Handling Policy

- Prefer fail-fast behavior by default.
- Avoid broad `do/catch` unless absolutely needed.
- If a catch block is used, handle the error explicitly, at minimum by logging it.

<!-- CONTENT -->

## Need More Than the Client?

If you need more about Rivet Actors, registries, or server-side RivetKit, add the main skill:

```bash
npx skills add rivet-dev/skills
```

Then use the `rivetkit` skill for backend guidance.
