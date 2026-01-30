# Deploying RivetKit to Netlify

Use this skill when deploying a RivetKit app to Netlify, configuring Netlify Functions with Rivet Actors, or connecting a Netlify site to the Rivet Engine.

## First Steps

1. Install RivetKit (latest: {{RIVETKIT_VERSION}})
   ```bash
   npm install rivetkit@{{RIVETKIT_VERSION}}
   ```
2. Prepare your project for Netlify (Next.js, Netlify Functions, or another supported framework).
3. Deploy to Netlify and set the required environment variables.
4. Connect your Netlify site to Rivet via the dashboard.

<!-- CONTENT -->

## Need More Than Deployment?

If you need guidance on building Rivet Actors, registries, or server-side RivetKit, add the main skill:

```bash
npx skills add rivet-dev/skills
```

Then use the `rivetkit` skill for backend guidance.
