import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import srvx from "vite-plugin-srvx";

export default defineConfig({
	plugins: [tailwindcss(), react(), ...srvx({ entry: "src/server.ts" })],
});
