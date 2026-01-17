import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import srvx from "vite-plugin-srvx";

export default defineConfig({
	plugins: [react(), ...srvx({ entry: "api/index.ts", prefix: "/api" })],
});
