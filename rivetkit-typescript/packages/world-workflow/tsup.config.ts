import { defineConfig } from "tsup";
import defaultConfig from "../../../tsup.base";

export default defineConfig({
	...defaultConfig,
	outDir: "dist/tsup/",
});
