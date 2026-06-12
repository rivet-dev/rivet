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
