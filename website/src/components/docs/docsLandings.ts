import {
	faCloudflare,
	faFastForward,
	faLayerGroup,
	faLightbulb,
	faNextjs,
	faNodeJs,
	faReact,
	faRust,
	faScaleBalanced,
} from "@rivet-gg/icons";
import { deployOptions, faSupabase } from "@rivetkit/shared-data";
import type { DocsLandingData } from "./DocsLanding";

const actors: DocsLandingData = {
	title: "Actors",
	subtitle:
		"Long-lived processes with durable state, realtime events, and built-in hibernation. Pick a stack to start building.",
	logo: "actors",
	sections: [
		{
			title: "Get Started",
			items: [
				{ title: "Node.js & Bun", href: "/docs/actors/quickstart/backend", icon: faNodeJs, description: "Set up actors with Node.js, Bun, and web frameworks." },
				{ title: "React", href: "/docs/actors/quickstart/react", icon: faReact, description: "Build realtime React applications backed by actors." },
				{ title: "Next.js", href: "/docs/actors/quickstart/next-js", icon: faNextjs, description: "Server-rendered Next.js experiences backed by actors." },
				{ title: "Rust", href: "/docs/actors/quickstart/rust", icon: faRust, badge: "Beta", description: "Native Rust with the typed rivetkit crate." },
				{ title: "Effect.ts", href: "/docs/actors/quickstart/effect", icon: faLayerGroup, badge: "Beta", description: "The Effect SDK with typed Schema actions." },
				{ title: "Cloudflare Workers", href: "/docs/actors/quickstart/cloudflare", icon: faCloudflare, description: "Run RivetKit on Cloudflare Workers." },
				{ title: "Supabase Functions", href: "/docs/actors/quickstart/supabase", icon: faSupabase, description: "Run RivetKit on Supabase Edge Functions." },
			],
		},
	],
};

const deploy: DocsLandingData = {
	title: "Deploy",
	subtitle: "Run RivetKit anywhere, from serverless functions to your own infrastructure.",
	sections: [
		{
			title: "Platforms",
			items: deployOptions.map((option) => ({
				title: option.shortTitle ?? option.displayName,
				href: option.href,
				icon: option.icon,
				badge: option.badge,
			})),
		},
	],
};

const agentOs: DocsLandingData = {
	title: "agentOS",
	subtitle:
		"Run coding agents inside isolated VMs with full filesystem, process, and network control.",
	logo: "agentos",
	sections: [
		{
			title: "Get Started",
			items: [
				{ title: "Quickstart", href: "/docs/agent-os/quickstart", icon: faFastForward, description: "Boot a VM and run your first coding agent." },
				{ title: "Crash Course", href: "/docs/agent-os/crash-course", icon: faLightbulb, description: "Learn the core agentOS concepts." },
				{ title: "agentOS vs Sandbox", href: "/docs/agent-os/versus-sandbox", icon: faScaleBalanced, description: "How agentOS compares to a plain sandbox." },
			],
		},
	],
};

export const docsLandings: Record<string, DocsLandingData> = {
	actors,
	deploy,
	"agent-os": agentOs,
};
