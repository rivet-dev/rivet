import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [react()],
	root: "src/frontend",
	server: {
		host: "0.0.0.0",
		port: 3000,
	},
	build: {
		outDir: "../../dist",
		emptyOutDir: true,
	},
});
