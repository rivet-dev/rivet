import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [react()],
	root: "frontend",
	build: {
		outDir: "dist",
		emptyOutDir: true,
	},
	server: {
		host: "0.0.0.0",
		port: 5173,
		proxy: {
			"/api/rivet/": "http://localhost:3000",
		},
	},
});
