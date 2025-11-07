import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App.tsx";
import styles from "./index.css?inline";

const root = document.createElement("div");
root.id = "rivetkit-inspector";

const shadow = root.attachShadow({ mode: "open" });
document.body.appendChild(root);

createRoot(shadow).render(
	<StrictMode>
		<style>{styles}</style>
		<App />
	</StrictMode>,
);
