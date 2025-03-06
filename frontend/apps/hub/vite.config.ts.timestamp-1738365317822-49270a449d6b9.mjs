// vite.config.ts
import * as crypto from "node:crypto";
import path from "node:path";
import { sentryVitePlugin } from "file:///home/rivet/rivet-ee/oss/node_modules/@sentry/vite-plugin/dist/esm/index.mjs";
import { transformerNotationFocus } from "file:///home/rivet/rivet-ee/oss/node_modules/@shikijs/transformers/dist/index.mjs";
import { TanStackRouterVite } from "file:///home/rivet/rivet-ee/oss/node_modules/@tanstack/router-vite-plugin/dist/esm/index.js";
import react from "file:///home/rivet/rivet-ee/oss/node_modules/@vitejs/plugin-react/dist/index.mjs";
import * as shiki from "file:///home/rivet/rivet-ee/oss/node_modules/shiki/dist/index.mjs";
import { defineConfig } from "file:///home/rivet/rivet-ee/oss/node_modules/vite/dist/node/index.js";
import vitePluginFaviconsInject from "file:///home/rivet/rivet-ee/oss/node_modules/vite-plugin-favicons-inject/dist/cjs/index.js";
var __vite_injected_original_dirname = "/home/rivet/rivet-ee/oss/frontend/apps/hub";
var GIT_BRANCH = process.env.CF_PAGES_BRANCH;
var GIT_SHA = process.env.CF_PAGES_COMMIT_SHA;
var vite_config_default = defineConfig({
  base: "./",
  plugins: [
    react(),
    TanStackRouterVite(),
    vitePluginFaviconsInject(
      path.resolve(__vite_injected_original_dirname, "public", "icon-white.svg"),
      {
        appName: "Rivet Hub",
        theme_color: "#ff4f00"
      }
    ),
    shikiTransformer(),
    process.env.SENTRY_AUTH_TOKEN ? sentryVitePlugin({
      org: "rivet-gaming",
      project: "hub",
      authToken: process.env.SENTRY_AUTH_TOKEN,
      release: GIT_BRANCH === "main" ? { name: GIT_SHA } : void 0
    }) : null
  ],
  server: {
    port: 5080
  },
  define: {
    // Provide a unique build ID for cache busting
    __APP_BUILD_ID__: JSON.stringify(
      `${(/* @__PURE__ */ new Date()).toISOString()}@${crypto.randomUUID()}`
    )
  },
  resolve: {
    alias: {
      "@": path.resolve(__vite_injected_original_dirname, "./src")
    }
  },
  build: {
    sourcemap: true,
    commonjsOptions: {
      include: [/@rivet-gg\/components/, /node_modules/]
    }
  },
  worker: {
    format: "es"
  }
});
async function shikiTransformer() {
  const cssVariableTheme = shiki.createCssVariablesTheme({
    name: "css-variables",
    variablePrefix: "--shiki-",
    variableDefaults: {},
    fontStyle: true
  });
  let highlighter;
  return {
    name: "shiki",
    async transform(code, id) {
      if (id.includes("?shiki")) {
        highlighter ??= await shiki.getSingletonHighlighter({
          themes: [cssVariableTheme],
          langs: [
            "bash",
            "batch",
            "cpp",
            "csharp",
            "docker",
            "gdscript",
            "html",
            "ini",
            "js",
            "json",
            "json",
            "powershell",
            "ts",
            "typescript",
            "yaml",
            "http",
            "prisma"
          ]
        });
        const params = new URLSearchParams(id.split("?")[1]);
        const output = highlighter.codeToHtml(code, {
          lang: params.get("lang") ?? "bash",
          theme: "css-variables",
          transformers: [transformerNotationFocus()]
        });
        return `export default ${JSON.stringify(
          output
        )};export const source = ${JSON.stringify(code)}`;
      }
    }
  };
}
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvaG9tZS9yaXZldC9yaXZldC1lZS9vc3MvZnJvbnRlbmQvYXBwcy9odWJcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIi9ob21lL3JpdmV0L3JpdmV0LWVlL29zcy9mcm9udGVuZC9hcHBzL2h1Yi92aXRlLmNvbmZpZy50c1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vaG9tZS9yaXZldC9yaXZldC1lZS9vc3MvZnJvbnRlbmQvYXBwcy9odWIvdml0ZS5jb25maWcudHNcIjtpbXBvcnQgKiBhcyBjcnlwdG8gZnJvbSBcIm5vZGU6Y3J5cHRvXCI7XG5pbXBvcnQgcGF0aCBmcm9tIFwibm9kZTpwYXRoXCI7XG5pbXBvcnQgeyBzZW50cnlWaXRlUGx1Z2luIH0gZnJvbSBcIkBzZW50cnkvdml0ZS1wbHVnaW5cIjtcbmltcG9ydCB7IHRyYW5zZm9ybWVyTm90YXRpb25Gb2N1cyB9IGZyb20gXCJAc2hpa2lqcy90cmFuc2Zvcm1lcnNcIjtcbmltcG9ydCB7IFRhblN0YWNrUm91dGVyVml0ZSB9IGZyb20gXCJAdGFuc3RhY2svcm91dGVyLXZpdGUtcGx1Z2luXCI7XG5pbXBvcnQgcmVhY3QgZnJvbSBcIkB2aXRlanMvcGx1Z2luLXJlYWN0XCI7XG5pbXBvcnQgKiBhcyBzaGlraSBmcm9tIFwic2hpa2lcIjtcbmltcG9ydCB7IHR5cGUgUGx1Z2luLCBkZWZpbmVDb25maWcgfSBmcm9tIFwidml0ZVwiO1xuaW1wb3J0IHZpdGVQbHVnaW5GYXZpY29uc0luamVjdCBmcm9tIFwidml0ZS1wbHVnaW4tZmF2aWNvbnMtaW5qZWN0XCI7XG5cbi8vIFRoZXNlIGFyZSBvbmx5IG5lZWRlZCBpbiBDSS4gVGhleSdsbCBiZSB1bmRlZmluZWQgaW4gZGV2LlxuY29uc3QgR0lUX0JSQU5DSCA9IHByb2Nlc3MuZW52LkNGX1BBR0VTX0JSQU5DSDtcbmNvbnN0IEdJVF9TSEEgPSBwcm9jZXNzLmVudi5DRl9QQUdFU19DT01NSVRfU0hBO1xuXG4vLyBodHRwczovL3ZpdGVqcy5kZXYvY29uZmlnL1xuZXhwb3J0IGRlZmF1bHQgZGVmaW5lQ29uZmlnKHtcblx0YmFzZTogXCIuL1wiLFxuXHRwbHVnaW5zOiBbXG5cdFx0cmVhY3QoKSxcblx0XHRUYW5TdGFja1JvdXRlclZpdGUoKSxcblx0XHR2aXRlUGx1Z2luRmF2aWNvbnNJbmplY3QoXG5cdFx0XHRwYXRoLnJlc29sdmUoX19kaXJuYW1lLCBcInB1YmxpY1wiLCBcImljb24td2hpdGUuc3ZnXCIpLFxuXHRcdFx0e1xuXHRcdFx0XHRhcHBOYW1lOiBcIlJpdmV0IEh1YlwiLFxuXHRcdFx0XHR0aGVtZV9jb2xvcjogXCIjZmY0ZjAwXCIsXG5cdFx0XHR9LFxuXHRcdCksXG5cdFx0c2hpa2lUcmFuc2Zvcm1lcigpLFxuXHRcdHByb2Nlc3MuZW52LlNFTlRSWV9BVVRIX1RPS0VOXG5cdFx0XHQ/IHNlbnRyeVZpdGVQbHVnaW4oe1xuXHRcdFx0XHRcdG9yZzogXCJyaXZldC1nYW1pbmdcIixcblx0XHRcdFx0XHRwcm9qZWN0OiBcImh1YlwiLFxuXHRcdFx0XHRcdGF1dGhUb2tlbjogcHJvY2Vzcy5lbnYuU0VOVFJZX0FVVEhfVE9LRU4sXG5cdFx0XHRcdFx0cmVsZWFzZTpcblx0XHRcdFx0XHRcdEdJVF9CUkFOQ0ggPT09IFwibWFpblwiID8geyBuYW1lOiBHSVRfU0hBIH0gOiB1bmRlZmluZWQsXG5cdFx0XHRcdH0pXG5cdFx0XHQ6IG51bGwsXG5cdF0sXG5cdHNlcnZlcjoge1xuXHRcdHBvcnQ6IDUwODAsXG5cdH0sXG5cdGRlZmluZToge1xuXHRcdC8vIFByb3ZpZGUgYSB1bmlxdWUgYnVpbGQgSUQgZm9yIGNhY2hlIGJ1c3Rpbmdcblx0XHRfX0FQUF9CVUlMRF9JRF9fOiBKU09OLnN0cmluZ2lmeShcblx0XHRcdGAke25ldyBEYXRlKCkudG9JU09TdHJpbmcoKX1AJHtjcnlwdG8ucmFuZG9tVVVJRCgpfWAsXG5cdFx0KSxcblx0fSxcblx0cmVzb2x2ZToge1xuXHRcdGFsaWFzOiB7XG5cdFx0XHRcIkBcIjogcGF0aC5yZXNvbHZlKF9fZGlybmFtZSwgXCIuL3NyY1wiKSxcblx0XHR9LFxuXHR9LFxuXHRidWlsZDoge1xuXHRcdHNvdXJjZW1hcDogdHJ1ZSxcblx0XHRjb21tb25qc09wdGlvbnM6IHtcblx0XHRcdGluY2x1ZGU6IFsvQHJpdmV0LWdnXFwvY29tcG9uZW50cy8sIC9ub2RlX21vZHVsZXMvXSxcblx0XHR9LFxuXHR9LFxuXHR3b3JrZXI6IHtcblx0XHRmb3JtYXQ6IFwiZXNcIixcblx0fSxcbn0pO1xuXG5hc3luYyBmdW5jdGlvbiBzaGlraVRyYW5zZm9ybWVyKCk6IFByb21pc2U8UGx1Z2luPiB7XG5cdGNvbnN0IGNzc1ZhcmlhYmxlVGhlbWUgPSBzaGlraS5jcmVhdGVDc3NWYXJpYWJsZXNUaGVtZSh7XG5cdFx0bmFtZTogXCJjc3MtdmFyaWFibGVzXCIsXG5cdFx0dmFyaWFibGVQcmVmaXg6IFwiLS1zaGlraS1cIixcblx0XHR2YXJpYWJsZURlZmF1bHRzOiB7fSxcblx0XHRmb250U3R5bGU6IHRydWUsXG5cdH0pO1xuXG5cdGxldCBoaWdobGlnaHRlcjogc2hpa2kuSGlnaGxpZ2h0ZXIgfCB1bmRlZmluZWQ7XG5cblx0cmV0dXJuIHtcblx0XHRuYW1lOiBcInNoaWtpXCIsXG5cdFx0YXN5bmMgdHJhbnNmb3JtKGNvZGUsIGlkKSB7XG5cdFx0XHRpZiAoaWQuaW5jbHVkZXMoXCI/c2hpa2lcIikpIHtcblx0XHRcdFx0aGlnaGxpZ2h0ZXIgPz89IGF3YWl0IHNoaWtpLmdldFNpbmdsZXRvbkhpZ2hsaWdodGVyKHtcblx0XHRcdFx0XHR0aGVtZXM6IFtjc3NWYXJpYWJsZVRoZW1lXSxcblx0XHRcdFx0XHRsYW5nczogW1xuXHRcdFx0XHRcdFx0XCJiYXNoXCIsXG5cdFx0XHRcdFx0XHRcImJhdGNoXCIsXG5cdFx0XHRcdFx0XHRcImNwcFwiLFxuXHRcdFx0XHRcdFx0XCJjc2hhcnBcIixcblx0XHRcdFx0XHRcdFwiZG9ja2VyXCIsXG5cdFx0XHRcdFx0XHRcImdkc2NyaXB0XCIsXG5cdFx0XHRcdFx0XHRcImh0bWxcIixcblx0XHRcdFx0XHRcdFwiaW5pXCIsXG5cdFx0XHRcdFx0XHRcImpzXCIsXG5cdFx0XHRcdFx0XHRcImpzb25cIixcblx0XHRcdFx0XHRcdFwianNvblwiLFxuXHRcdFx0XHRcdFx0XCJwb3dlcnNoZWxsXCIsXG5cdFx0XHRcdFx0XHRcInRzXCIsXG5cdFx0XHRcdFx0XHRcInR5cGVzY3JpcHRcIixcblx0XHRcdFx0XHRcdFwieWFtbFwiLFxuXHRcdFx0XHRcdFx0XCJodHRwXCIsXG5cdFx0XHRcdFx0XHRcInByaXNtYVwiLFxuXHRcdFx0XHRcdF0sXG5cdFx0XHRcdH0pO1xuXG5cdFx0XHRcdGNvbnN0IHBhcmFtcyA9IG5ldyBVUkxTZWFyY2hQYXJhbXMoaWQuc3BsaXQoXCI/XCIpWzFdKTtcblx0XHRcdFx0Y29uc3Qgb3V0cHV0ID0gaGlnaGxpZ2h0ZXIuY29kZVRvSHRtbChjb2RlLCB7XG5cdFx0XHRcdFx0bGFuZzogcGFyYW1zLmdldChcImxhbmdcIikgPz8gXCJiYXNoXCIsXG5cdFx0XHRcdFx0dGhlbWU6IFwiY3NzLXZhcmlhYmxlc1wiLFxuXHRcdFx0XHRcdHRyYW5zZm9ybWVyczogW3RyYW5zZm9ybWVyTm90YXRpb25Gb2N1cygpXSxcblx0XHRcdFx0fSk7XG5cdFx0XHRcdHJldHVybiBgZXhwb3J0IGRlZmF1bHQgJHtKU09OLnN0cmluZ2lmeShcblx0XHRcdFx0XHRvdXRwdXQsXG5cdFx0XHRcdCl9O2V4cG9ydCBjb25zdCBzb3VyY2UgPSAke0pTT04uc3RyaW5naWZ5KGNvZGUpfWA7XG5cdFx0XHR9XG5cdFx0fSxcblx0fTtcbn1cbiJdLAogICJtYXBwaW5ncyI6ICI7QUFBZ1QsWUFBWSxZQUFZO0FBQ3hVLE9BQU8sVUFBVTtB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
