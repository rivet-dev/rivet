import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base.ts";

export default defineConfig({
	external: [/cloudflare:.*/],
	...defaultConfig,
});
