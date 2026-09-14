import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { getConfig, getPosthogConfig } from "@/components/lib/config";
import { maybeStartAgentMocks } from "./lib/agent-mocks";
import { snapshotPosthogFeatureFlags } from "./lib/posthog";

async function init() {
	await maybeStartAgentMocks();

	// PostHog flags additively extend `features`, which several modules read at
	// import time and which must stay constant once components mount. The
	// snapshot therefore has to complete before the app module graph is
	// evaluated, which is what keeps the imports below dynamic.
	const posthogConfig = getPosthogConfig(getConfig());
	if (posthogConfig) {
		await snapshotPosthogFeatureFlags(
			posthogConfig.apiKey,
			posthogConfig.apiHost,
			false,
		);
	}

	const [
		{ initThirdPartyProviders },
		{ App, router },
		{ restoreQueryCache },
	] = await Promise.all([
		import("@/components"),
		import("./app"),
		import("./queries/global"),
	]);

	await initThirdPartyProviders(router, false);

	// Rehydrate the persisted query cache before the router mounts so cache-first
	// loaders resolve from localStorage on first paint instead of blocking.
	await restoreQueryCache();

	// biome-ignore lint/style/noNonNullAssertion: it should always be present
	const rootElement = document.getElementById("root")!;
	if (!rootElement.innerHTML) {
		const root = ReactDOM.createRoot(rootElement);
		root.render(
			<StrictMode>
				<App router={router} />
			</StrictMode>,
		);
	}
}

init().catch(console.error);
