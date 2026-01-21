import path from "node:path";

import { defineConfig } from "tsup";

const sdkBase = path.resolve(
	__dirname,
	"../../../node_modules/.pnpm/@modelcontextprotocol+sdk@1.25.3_hono@4.11.3_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm",
);

export default defineConfig({
	entry: ["src/index.ts", "src/cli.ts"],
	format: ["esm"],
	dts: true,
	sourcemap: true,
	clean: true,
	target: "node20",
	alias: {
		"@modelcontextprotocol/sdk": path.join(sdkBase, "index.js"),
		"@modelcontextprotocol/sdk/server/streamableHttp.js": path.join(sdkBase, "server/streamableHttp.js"),
		"@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js": path.join(
			sdkBase,
			"server/webStandardStreamableHttp.js",
		),
	},
});
