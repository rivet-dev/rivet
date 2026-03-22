import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
	plugins: [react()],
	publicDir: false,
	build: {
		outDir: "public",
		emptyOutDir: true,
	},
	server: {
		clearScreen: false,
		proxy: {
			"/actors": { target: "http://localhost:6420", ws: true },
			"/metadata": { target: "http://localhost:6420" },
			"/health": { target: "http://localhost:6420" },
		},
	},
});
