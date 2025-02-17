import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [react(), dts({ include: "src", insertTypesEntry: true })],
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
		},
	},
	build: {
		sourcemap: true,
		lib: {
			entry: {
				index: path.resolve(__dirname, "src/index.ts"),
				"code-mirror": path.resolve(
					__dirname,
					"src/code-mirror/index.tsx",
				),
				"tailwind-base": path.resolve(
					__dirname,
					"src/tailwind-base.ts",
				),
				mdx: path.resolve(__dirname, "src/mdx/index.tsx"),
			},
		},
		rollupOptions: {
			external: [
				"react",
				"react/jsx-runtime",
				"react-hook-form",
				"zod",
				"@rivet-gg/icons",
				"@rivet-gg/api",
				"@fortawesome/fontawesome-svg-core",
				"@fortawesome/free-brands-svg-icons",
				"@fortawesome/free-solid-svg-icons",
				"@fortawesome/react-fontawesome",
			],
		},
	},
});
