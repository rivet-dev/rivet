# Deploying to Railway

> Source: `src/content/docs/connect/railway.mdx`
> Canonical URL: https://rivet.gg/docs/connect/railway
> Description: Deploy your RivetKit app to [Railway](https://railway.app).

---
## Steps

### Prerequisites

- [Railway account](https://railway.app)
- Your RivetKit app
  - If you don't have one, see the [Quickstart](/docs/actors/quickstart) page or our [Examples](https://github.com/rivet-dev/rivet/tree/main/examples)
- Access to the [Rivet Cloud](https://dashboard.rivet.dev/) or a [self-hosted Rivet Engine](/docs/general/self-hosting)

### Deploy to Railway

1. Connect your GitHub account to Railway
2. Select your repository containing your RivetKit app
3. Railway will automatically detect and deploy your app

See [Railway's deployment docs](https://docs.railway.com/quick-start) for more details.

### Set Environment Variables

After creating your project on the Rivet dashboard, select Railway as your provider. You'll be provided `RIVET_ENDPOINT` and `RIVET_PUBLIC_ENDPOINT` environment variables to add to your Railway service.

See [Railway's environment variables docs](https://docs.railway.com/guides/variables#service-variables) for more details.

### Connect to Rivet

1. In your Railway project, go to **Settings > Networking**
2. Click **Create Custom Domain** then **Create Domain** to generate a Railway domain (e.g. `my-app.railway.app`)
3. On the Rivet dashboard, paste your domain with the `/api/rivet` path into the connect form (e.g. `https://my-app.railway.app/api/rivet`)
4. Click "Done"

### Configure Sleeping & Multi-Region (Optional)

- [Enable App Sleeping](https://docs.railway.com/reference/app-sleeping) to reduce costs when idle
- [Configure Multi-Region](https://docs.railway.com/reference/deployment-regions) to deploy closer to your users

_Source doc path: /docs/connect/railway_
