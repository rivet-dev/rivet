// Single source of truth for SEO redirects.
//
// Imported by `astro.config.mjs` (so the dev server and the static HTML
// fallback pages stay in sync) and by `scripts/generate-caddy-redirects.mjs`
// (which emits real HTTP 301s at the Caddy layer for production).
//
// Keys and values are path-only (no origin). Targets should end in `/` to
// match the site's canonical trailing-slash form and avoid a second redirect
// hop.
export const redirects = {
	'/docs': '/docs/actors/',
	// Documentation restructure
	'/docs/setup': '/docs/actors/quickstart/',
	'/docs/actors/queue': '/docs/actors/queues/',
	'/docs/actors/websockets': '/docs/actors/websocket-handler/',
	'/docs/actors/http': '/docs/actors/http-api/',
	'/docs/actors/run': '/docs/actors/lifecycle/',
	'/docs/actors/scheduling': '/docs/actors/schedule/',
	'/docs/actors/external-sql': '/docs/actors/state/',
	'/docs/actors/raw-sql': '/docs/actors/sqlite/',
	'/docs/actors/ephemeral-variables': '/docs/actors/state/',
	'/docs/actors/persistence': '/docs/actors/state/',
	'/docs/actors/postgres': '/docs/actors/state/',
	// Platform docs moved to clients/connect
	'/docs/platforms/react': '/docs/clients/react/',
	'/docs/platforms/next-js': '/docs/clients/javascript/',
	// Registry configuration moved
	'/docs/connect/registry-configuration': '/docs/general/registry-configuration/',
	// Quickstart index merged into the Actors introduction
	'/docs/actors/quickstart': '/docs/actors/',
	// Connect tab renamed to Deploy
	'/docs/connect': '/docs/deploy/',
	'/docs/connect/aws-ecs': '/docs/deploy/aws-ecs/',
	'/docs/connect/aws-lambda': '/docs/deploy/aws-lambda/',
	'/docs/connect/cloudflare': '/docs/deploy/cloudflare/',
	'/docs/connect/custom': '/docs/deploy/custom/',
	'/docs/connect/freestyle': '/docs/deploy/freestyle/',
	'/docs/connect/gcp-cloud-run': '/docs/deploy/gcp-cloud-run/',
	'/docs/connect/hetzner': '/docs/deploy/hetzner/',
	'/docs/connect/kubernetes': '/docs/deploy/kubernetes/',
	'/docs/connect/railway': '/docs/deploy/railway/',
	'/docs/connect/rivet-compute': '/docs/deploy/rivet-compute/',
	'/docs/connect/supabase': '/docs/deploy/supabase/',
	'/docs/connect/vercel': '/docs/deploy/vercel/',
	'/docs/connect/vm-and-bare-metal': '/docs/deploy/vm-and-bare-metal/',
	// Cloud docs removed - redirect to relevant sections
	'/docs/cloud': '/docs/self-hosting/',
	'/docs/cloud/api/actors/create': '/docs/actors/',
	'/docs/cloud/api/routes/update': '/docs/actors/',
	'/docs/cloud/self-hosting/single-container': '/docs/self-hosting/docker-container/',
	// Next.js client redirect (linked from homepage)
	'/docs/clients/next-js': '/docs/clients/javascript/',
	// Self-hosting redirect
	'/docs/general/self-hosting': '/docs/self-hosting/',
	// Removed solution pages
	'/solutions/agents': '/',
	'/solutions/app-generators': '/',
	'/solutions/collaborative-state': '/',
	'/solutions/game-servers': '/',
	'/solutions/games': '/',
	'/solutions/geo-distributed-db': '/',
	'/solutions/per-tenant-db': '/',
	'/solutions/user-session-store': '/',
	'/solutions/workflows': '/',
	// Changelog list view merged into the blog index
	'/changelog': '/blog/',
};
