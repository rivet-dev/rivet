# RivetKit + Convex Counter

A real-time counter example demonstrating RivetKit actors running on [Convex](https://convex.dev/).

## Getting Started

### 1. Install dependencies

```bash
pnpm install
```

### 2. Deploy to Convex

```bash
npx convex dev
```

Note your deployment name (e.g., `cool-lobster-787`). Your site URL is `https://{deployment-name}.convex.site`.

### 3. Create a Rivet Cloud project

1. Go to [dashboard.rivet.gg](https://dashboard.rivet.gg)
2. Click **Start From Scratch**
3. Select **Vercel** as the platform (Convex support coming soon)
4. Set the environment variables on your Convex deployment:
   - `RIVET_ENDPOINT` - The endpoint provided by Rivet Cloud
   - `RIVET_PUBLIC_ENDPOINT` - The public endpoint provided by Rivet Cloud
5. Enter your Convex site URL (e.g., `https://your-deployment.convex.site`)
6. Click **Advanced** and set the timeout to **570 seconds** (Convex actions have a 10-minute limit)
7. Verify the connection and click **Next** to complete setup

### 4. Run the frontend

```bash
pnpm dev
```

This automatically picks up the `VITE_CONVEX_URL` environment variable that `convex dev` writes to `.env.local`.

## Deployment

When deploying your frontend to production, you'll need to manually set the `VITE_CONVEX_URL` environment variable to your Convex deployment URL (e.g., `https://your-deployment.convex.cloud`).

## How It Works

```
Browser → Rivet Cloud → Convex (serverless)
              ↑                    ↑
         Manages actors     Runs actor logic
         and state
```

## Implementation

This example demonstrates the integration structure for RivetKit with Convex.

The key components are:

1. **Actor definitions** in [`convex/rivet/actors.ts`](https://github.com/rivet-gg/rivet/tree/main/examples/convex/convex/rivet/actors.ts) define the counter actor with `increment` and `getCount` actions.

2. **HTTP routing** in [`convex/http.ts`](https://github.com/rivet-gg/rivet/tree/main/examples/convex/convex/http.ts) routes all `/api/rivet/` requests to the Node.js action handler using `addRivetRoutes`.

3. **Node.js action** in [`convex/rivet.ts`](https://github.com/rivet-gg/rivet/tree/main/examples/convex/convex/rivet.ts) handles RivetKit requests using `createRivetAction` from `@rivetkit/convex`.

## Why Node.js Actions?

Convex HTTP actions run in a V8 isolate that doesn't include the WebSocket API. RivetKit's engine driver requires WebSocket to communicate with Rivet Cloud for actor state management.

Convex Node.js actions have access to the full Node.js runtime, including the `ws` package for WebSocket support. This example routes HTTP requests through a Node.js action to enable WebSocket connectivity.

## Resources

- [RivetKit documentation](https://rivet.gg/docs)
- [Convex HTTP Actions documentation](https://docs.convex.dev/functions/http-actions)
- [Rivet Cloud Dashboard](https://dashboard.rivet.gg/)

## License

MIT
