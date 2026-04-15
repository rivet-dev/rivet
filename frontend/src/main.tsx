import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import type { Clerk } from "@clerk/clerk-js";
import { initThirdPartyProviders } from "@/components";
import { clerkPromise } from "@/lib/auth";
import { App, createAppRouter } from "./app";

async function init() {
	const clerk = await clerkPromise.catch((error) => {
		console.error("Failed to initialize Clerk", error);
		return null as unknown as Clerk;
	});
	const router = createAppRouter(clerk);

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
