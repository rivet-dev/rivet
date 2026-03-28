import { sentryVitePlugin } from "@sentry/vite-plugin";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import favigo from "favigo/vite";
import Macros from "unplugin-macros/vite";
import { defineConfig, loadEnv, mergeConfig, type Plugin } from "vite";
import { commonEnvSchema } from "./src/lib/env";
import { baseViteConfig } from "./vite.base.config";

// These are only needed in CI. They'll be undefined in dev.
const GIT_BRANCH = process.env.CF_PAGES_BRANCH;
const GIT_SHA = process.env.CF_PAGES_COMMIT_SHA;

const getVariantForMode = (mode: string) => {
	switch (mode) {
		case "staging":
			return {
				type: "badge",
				text: "DEV",
				backgroundColor: "#FF4F00",
				textColor: "#ffffff",
				position: "bottom-right",
				size: "large",
			} as const;
		default:
			return undefined;
	}
};

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
	const env = commonEnvSchema.parse(loadEnv(mode, process.cwd(), ""));

	console.log(
		env.SENTRY_AUTH_TOKEN
			? "Sentry plugin enabled"
			: "Sentry plugin disabled (missing auth token)",
	);

	return mergeConfig(baseViteConfig(), {
		base: "/ui",
		plugins: [
			iconsSideEffectsFreePlugin(),
			clerkSideEffectsFreePlugin(),
			posthogStubPlugin(),
			cloudRoutesStubPlugin(),
			tanstackRouter({ target: "react", autoCodeSplitting: true }),
			react({
				exclude: [/packages\/icons\/dist/],
				babel: {
					plugins: ["babel-plugin-react-compiler"],
				},
			}),
			liveChatPlugin(),
			env.SENTRY_AUTH_TOKEN
				? sentryVitePlugin({
						org: "rivet-gaming",
						project: env.SENTRY_PROJECT,
						authToken: env.SENTRY_AUTH_TOKEN,
						release:
							GIT_BRANCH === "main"
								? { name: GIT_SHA }
								: undefined,
					})
				: null,
			favigo({
				source: "./public/favicon.svg",
				variant: getVariantForMode(env.DEPLOYMENT_TYPE || "production"),
				configuration: {
					theme_color: "#FF4F00",
					background: "transparent",
				},
			}),
			Macros(),
		],
		server: {
			port: 43708,
			proxy: {
				"/api": {
					target: "http://localhost:6420",
					changeOrigin: true,
					rewrite: (path: string) => path.replace(/^\/api/, ""),
				},
			},
		},
		preview: {
			port: 43708,
		},
		define: {
			__APP_TYPE__: JSON.stringify(env.APP_TYPE || "engine"),
		},
		build: {
			sourcemap: true,
			commonjsOptions: {
				include: [/@rivet-gg\/components/, /node_modules/],
			},
			rollupOptions: {
				treeshake: {
					propertyReadSideEffects: false,
				},
			},
		},
	});
});

export function liveChatPlugin(source: string = ""): Plugin {
	return {
		name: "live-chat-plugin",
		transformIndexHtml(html) {
			return html.replace(/{{live_chat}}/, source);
		},
	};
}

// Marks @clerk/* packages as side-effect-free so Rollup tree-shakes them in the engine build,
// where all Clerk usage is guarded by __APP_TYPE__ === "cloud" conditions.
function clerkSideEffectsFreePlugin(): Plugin {
	return {
		name: "clerk-side-effects-free",
		enforce: "pre",
		async resolveId(id, importer, options) {
			if (id.startsWith("@clerk/")) {
				const resolved = await this.resolve(id, importer, {
					...options,
					skipSelf: true,
				});
				if (resolved) {
					return { ...resolved, moduleSideEffects: false };
				}
			}
		},
		transform(code, id) {
			if (id.includes("/node_modules/@clerk/")) {
				return { code, map: null, moduleSideEffects: false };
			}
		},
	};
}

// Replaces all cloud-only route files with minimal stubs in the engine build, eliminating
// cloud-specific code (Clerk UI, PostHog, billing components, etc.) from the bundle entirely.
//
// TanStack Router's routeTree.gen.ts always patches imported routes via .update({ id, path,
// getParentRoute }), so stubs only need to export a valid Route object. The path passed to
// createFileRoute() is overridden and does not matter.
function cloudRoutesStubPlugin(): Plugin {
	// Files to stub: the _cloud layout, all _cloud child routes, and standalone cloud-only routes.
	const CLOUD_ROUTE_RE =
		/\/routes\/_context\/_cloud(\.tsx|\/)|\/routes\/(sso-callback|onboarding\/accept-invitation)\.tsx$/;

	// autoCodeSplitting generates lazy split chunks that import RouteComponent (and possibly
	// PendingComponent / ErrorComponent) from the original route file by name. The stub must
	// export those names so the generated split imports resolve without errors.
	const STUB_CODE = `
import { createFileRoute } from "@tanstack/react-router";
export function RouteComponent() { return null; }
export function PendingComponent() { return null; }
export function ErrorComponent() { return null; }
export const Route = createFileRoute("/__cloud-stub")({ component: RouteComponent });
`;

	return {
		name: "cloud-routes-stub",
		enforce: "pre",
		load(id) {
			if (CLOUD_ROUTE_RE.test(id)) {
				return STUB_CODE;
			}
		},
	};
}

// Replaces posthog-js with a no-op stub in the engine build so posthog code is excluded from
// the bundle. Cloud-only files may import posthog-js statically; this prevents them from pulling
// the full 5% posthog bundle into the engine build.
function posthogStubPlugin(): Plugin {
	const VIRTUAL_PREFIX = "\0posthog-stub:";
	// A Proxy-based stub: every property access returns itself (callable no-op).
	// Exports both default and named "posthog" to satisfy all import styles.
	const STUB_CODE = `
const _handler = { get: () => _noop, apply: () => undefined, construct: () => ({}) };
function _noop() {}
const posthog = new Proxy(_noop, _handler);
export default posthog;
export { posthog };
export const usePostHog = () => posthog;
export const PostHogProvider = ({ children }) => children;
`;
	return {
		name: "posthog-stub",
		enforce: "pre",
		resolveId(id) {
			if (id === "posthog-js" || id === "posthog-js/react") {
				return `${VIRTUAL_PREFIX}${id}`;
			}
		},
		load(id) {
			if (id.startsWith(VIRTUAL_PREFIX)) {
				return STUB_CODE;
			}
		},
	};
}

// Marks the @rivet-gg/icons flat ESM as side-effect-free so Rollup tree-shakes unused icons.
function iconsSideEffectsFreePlugin(): Plugin {
	return {
		name: "icons-side-effects-free",
		enforce: "pre",
		async resolveId(id, importer, options) {
			if (id === "@rivet-gg/icons") {
				const resolved = await this.resolve(id, importer, {
					...options,
					skipSelf: true,
				});
				if (resolved) {
					return { ...resolved, moduleSideEffects: false };
				}
			}
		},
		transform(code, id) {
			if (id.includes("packages/icons/dist/index.flat")) {
				return { code, map: null, moduleSideEffects: false };
			}
		},
	};
}
