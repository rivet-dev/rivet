import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import srvx from "vite-plugin-srvx";

export default defineConfig({
	plugins: [
		react(),
		...srvx({ entry: "src/server.ts", clientOutDir: "dist/public" }),
	],
	// Keep RivetKit's native runtime imports relative to its installed package.
	ssr: { external: ["rivetkit"] },
});
