// Single source of truth for SEO redirects.
//
// Imported by `astro.config.mjs` (so the dev server and the static HTML
// fallback pages stay in sync) and by `scripts/generate-caddy-redirects.mjs`
// (which emits real HTTP 301s at the Caddy layer for production).
//
// Keys and values are path-only (no origin) for internal redirects. Internal
// targets should end in `/` to match the site's canonical trailing-slash form
// and avoid a second redirect hop.
//
// External redirects to the agentOS site (`https://agentos-sdk.dev`) are also
// supported. agentOS was split out into its own site, so every `/agent-os` and
// `/docs/agent-os` path redirects out to it. See `EXTERNAL_REDIRECT_HOST` and
// `wildcardRedirects` below, and the matching handling in
// `scripts/generate-caddy-redirects.mjs`.
export const redirects = {
	'/docs': '/docs/actors/',
	// Integrations moved out of the documentation URL hierarchy.
	'/docs/integrations': '/integrations/',
	'/docs/integrations/vercel-eve': '/integrations/vercel-eve/',
	'/docs/integrations/vercel-workflow': '/integrations/vercel-workflows/',
	'/integrations/vercel-workflow': '/integrations/vercel-workflows/',
	// Documentation restructure
	'/docs/setup': '/docs/actors/quickstart/',
	'/docs/deploy/cli': '/docs/cli/',
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
	'/agent': '/actors/',
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
	// agentOS moved to its own site at https://agentos-sdk.dev.
	// Marketing pages and the former "From Unix to Agents" essay all point at the
	// new site root. Per-page marketing paths do not have a clean 1:1 mapping on
	// the new site, so the whole `/agent-os` prefix collapses to the root.
	'/agent-os': 'https://agentos-sdk.dev',
	'/agent-os/pricing': 'https://agentos-sdk.dev',
	'/agent-os/use-cases': 'https://agentos-sdk.dev',
	'/agent-os/registry': 'https://agentos-sdk.dev',
	'/from-unix-to-agents': 'https://agentos-sdk.dev',
	'/install': 'https://agentos-sdk.dev',
	'/registry': 'https://agentos-sdk.dev',
	// agentOS docs collapse to the new site root.
	'/docs/agent-os': 'https://agentos-sdk.dev',
	// The agentOS workspace cookbook moved with the rest of agentOS.
	'/cookbook/ai-agent-workspace': 'https://agentos-sdk.dev',
};

// External host that wildcard and absolute-URL redirect targets are restricted
// to. Used by both the Astro config and the Caddy generator so neither consumer
// can accidentally emit a redirect to an arbitrary host.
export const EXTERNAL_REDIRECT_HOST = 'agentos-sdk.dev';

// Wildcard (prefix) redirects. Any request under `from` (at any depth) is sent
// to `to`. agentOS moved to its own site as a single destination, so every
// `/agent-os/*` and `/docs/agent-os/*` sub-path collapses to the new site root
// rather than mapping its suffix through. The `/agent-os` prefix subsumes
// `/agent-os/registry/*` and any other deep marketing path.
export const wildcardRedirects = [
	{ from: '/agent-os', to: 'https://agentos-sdk.dev' },
	{ from: '/docs/agent-os', to: 'https://agentos-sdk.dev' },
];
