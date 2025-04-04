import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { App, router } from "./app";
import "./index.css";
import { rivetClient } from "./queries/global";
import { initThirdPartyProviders } from "@rivet-gg/components";

initThirdPartyProviders(router, import.meta.env.DEV);

rivetClient.cloud
	.bootstrap()
	.then((response) => {
		run({ cacheKey: [response.deployHash, __APP_BUILD_ID__].join("-") });
	})
	.catch(() => {
		run();
	});

function run({ cacheKey }: { cacheKey?: string } = {}) {
	// biome-ignore lint/style/noNonNullAssertion: it should always be present
	const rootElement = document.getElementById("root")!;
	if (!rootElement.innerHTML) {
		const root = ReactDOM.createRoot(rootElement);
		root.render(
			<StrictMode>
				<App cacheKey={cacheKey} />
			</StrictMode>,
		);
	}
}
