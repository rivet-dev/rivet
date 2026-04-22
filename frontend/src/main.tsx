import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { initThirdPartyProviders } from "@/components";
import { App, router } from "./app";

async function init() {
	await initThirdPartyProviders(router, false);

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
