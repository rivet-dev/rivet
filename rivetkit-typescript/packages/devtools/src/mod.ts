import svg from "./icon.svg";
import styles from "./styles.css";

declare global {
	interface Window {
		_rivetkit_devtools_configs?: Array<
			Parameters<typeof import("rivetkit/client")["createClient"]>[0]
		>;
	}
}

const root = document.createElement("div");

root.id = "rivetkit-devtools";
const shadow = root.attachShadow({ mode: "open" });

const div = document.createElement("div");

const img = document.createElement("img");
img.src = svg;
div.appendChild(img);

const btn = document.createElement("button");
btn.appendChild(div);

const tooltip = document.createElement("div");
tooltip.className = "tooltip";
tooltip.textContent = "Open Inspector";

const style = document.createElement("style");
style.textContent = styles;
shadow.appendChild(style);
shadow.appendChild(btn);
shadow.appendChild(tooltip);

btn.addEventListener("mouseenter", () => {
	tooltip.classList.add("visible");
});

btn.addEventListener("mouseleave", () => {
	tooltip.classList.remove("visible");
});

btn.addEventListener("click", () => {
	const config = window._rivetkit_devtools_configs?.[0];
	if (!config || typeof config !== "object") {
		console.error("RivetKit Devtools: No client config found");
		return;
	}
	const url = new URL("https://inspect.rivet.dev/");
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
});

document.body.appendChild(root);
