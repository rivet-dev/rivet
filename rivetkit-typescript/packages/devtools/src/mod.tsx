import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { DevButton } from "./components/DevButton";
import svg from "./icon.svg";
import styles from "./styles.css";

declare global {
	interface Window {
		__rivetkit?: Array<
			Parameters<typeof import("rivetkit/client")["createClient"]>[0]
		>;
	}
}

const root = document.createElement("rivetkit-devtools");
root.style.zIndex = "2147483647";
root.style.pointerEvents = "none";
const shadow = root.attachShadow({ mode: "open" });

createRoot(shadow).render(
	<StrictMode>
		<App />
		<style
			/** biome-ignore lint/security/noDangerouslySetInnerHtml: it's okay */
			dangerouslySetInnerHTML={{ __html: styles }}
		/>
	</StrictMode>,
);

function App() {
	return (
		<DevButton
			onClick={() => {
				openDevtools();
			}}
		>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					justifyContent: "center",
					width: 48,
					height: 48,
				}}
			>
				<img
					src={svg}
					style={{ pointerEvents: "none" }}
					alt="RivetKit Devtools"
				/>
			</div>
		</DevButton>
	);
}

const openDevtools = () => {
	const config = window.__rivetkit?.[0];
	if (!config || typeof config !== "object") {
		console.error("RivetKit Devtools: No client config found");
		return;
	}
	const url = new URL("http://localhost:6420/ui");
	if (!config.endpoint) {
		console.error("RivetKit Devtools: No endpoint found in client config");
		return;
	}
	url.searchParams.set("u", config.endpoint);
	if (config.token) {
		url.searchParams.set("t", config.token);
	}
	if (config.namespace) {
		url.searchParams.set("ns", config.namespace);
	}
	if (config.runnerName) {
		url.searchParams.set("r", config.runnerName);
	}
	window.open(url.toString(), "_blank");
};

document.body.appendChild(root);
