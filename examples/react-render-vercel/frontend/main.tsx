import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "render-dds";
import { App } from "./app/App.tsx";
import "./app.css";

const root = document.getElementById("root");
if (!root) throw new Error("Root element not found");

createRoot(root).render(
	<StrictMode>
		<ThemeProvider defaultTheme="light" enableSystem>
			<App />
		</ThemeProvider>
	</StrictMode>,
);
