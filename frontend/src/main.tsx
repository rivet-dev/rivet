import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { initThirdPartyProviders } from "@/components";
import { App, router } from "./app";
import { maybeStartAgentMocks } from "./lib/agent-mocks";
import { restoreQueryCache } from "./queries/global";

async function init() {
	await maybeStartAgentMocks();
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
