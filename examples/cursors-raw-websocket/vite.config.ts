import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import srvx from "vite-plugin-srvx";

export default defineConfig({
	plugins: [react(), ...srvx({ entry: "src/server.ts" })],
});
