import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [react()],
	root: "frontend",
	build: {
		emptyOutDir: true,
	},
	server: {
		host: "0.0.0.0",
		port: 5173,
		proxy: {
			"/api/": "http://localhost:3001",
		},
	},
});
