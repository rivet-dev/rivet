/// <reference types="@types/node" />

import { defineConfig } from "tsup";

export default defineConfig({
    target: "esnext",
    platform: "browser",
    format: ["esm"],
    sourcemap: true,
    clean: false, // Don't clean since main build runs first
    dts: {
        compilerOptions: {
            skipLibCheck: true,
            resolveJsonModule: true,
            lib: ["ESNext", "DOM"],
        },
    },
    minify: false,
    // CRITICAL: Disable code splitting to prevent shared chunks with Node.js code
    splitting: false,
    skipNodeModulesBundle: true,
    external: [
        // Mark all Node.js-only packages as external
        /^node:.*/,
        "@hono/node-server",
        "@hono/node-server/serve-static",
        "@hono/node-ws",
        "tar",
        "module",
        // Keep workspace packages external
        "@rivetkit/traces",
        "@rivetkit/traces/encoding",
        "@rivetkit/traces/otlp",
        "@rivetkit/workflow-engine",
    ],
    // No shims needed for browser builds
    shims: false,
    outDir: "dist/browser/",
    entry: {
        "inspector/client": "src/inspector/mod.browser.ts",
        "client": "src/client/mod.browser.ts",
    },
    define: {
        "globalThis.CUSTOM_RIVETKIT_DEVTOOLS_URL": process.env
            .CUSTOM_RIVETKIT_DEVTOOLS_URL
            ? `"${process.env.CUSTOM_RIVETKIT_DEVTOOLS_URL}"`
            : "false",
    },
});
