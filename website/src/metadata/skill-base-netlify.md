# Deploying RivetKit to Netlify

Use this skill when deploying a RivetKit app to Netlify or connecting a Netlify site to the Rivet Engine.

## First Steps

1. If you don't have a RivetKit app yet, clone the hello-world example:
   ```bash
   npx giget@latest gh:rivet-dev/rivet/examples/hello-world hello-world --install
   ```
2. Install the Netlify Functions package:
   ```bash
   npm install @netlify/functions
   ```
3. Add a `netlify.toml` with build, functions, and redirect configuration. Set `publish` to match your frontend build output directory (e.g. `public` for the hello-world example).
4. Add a `functions/rivet.ts` handler that converts Netlify events to standard `Request` objects and forwards them to your Hono app via `app.fetch(request)`.
5. Set environment variables (`RIVET_ENDPOINT`, `RIVET_PUBLIC_ENDPOINT`) in the Netlify dashboard.
6. Deploy to Netlify and connect your site via the Rivet dashboard.

<!-- CONTENT -->

## Need More Than Deployment?

If you need guidance on building Rivet Actors, registries, or server-side RivetKit, add the main skill:

```bash
npx skills add rivet-dev/skills
```

Then use the `rivetkit` skill for backend guidance.
