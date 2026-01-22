import { defineConfig, mergeConfig } from "vitest/config";
import defaultConfig from "../../../vitest.base.ts";

export default mergeConfig(defaultConfig, defineConfig({}));
