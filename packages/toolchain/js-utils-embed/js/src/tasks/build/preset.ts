import type { Preset } from "unenv";

// Built-in APIs provided by workerd.
//
// https://developers.cloudflare.com/workers/runtime-apis/nodejs/
// https://github.com/cloudflare/workerd/tree/main/src/node
//
// Last checked: 2025-01-24
//
// NOTE: Please sync any changes to `testNodeCompatModules`.
const nodeCompatModules = [
	//"_stream_duplex",
	//"_stream_passthrough",
	//"_stream_readable",
	//"_stream_transform",
	//"_stream_writable",
	//"assert",
	//"assert/strict",
	//"buffer",
	//"diagnostics_channel",
	//"dns",
	//"dns/promises",
	//"events",
	//"net",
	//"path",
	//"path/posix",
	//"path/win32",
	//"querystring",
	//"stream",
	//"stream/consumers",
	//"stream/promises",
	//"stream/web",
	//"string_decoder",
	//"timers",
	//"timers/promises",
	//"url",
	//"util/types",
	//"zlib",
];

// Modules implemented via a mix of workerd APIs and polyfills.
const hybridNodeCompatModules = [
	"async_hooks",
	"console",
	"crypto",
	"module",
	"process",
	"util",
];

export const rivetPreset: Preset = {
	meta: {
		name: "unenv:rivet",
		//url: __filename,
	},
	alias: {
		//...Object.fromEntries(
		//	nodeCompatModules.flatMap((p) => [
		//		[p, p],
		//		[`node:${p}`, `node:${p}`],
		//	])
		//),
		//
		//// The `node:sys` module is just a deprecated alias for `node:util` which we implemented using a hybrid polyfill
		//sys: "@cloudflare/unenv-preset/runtime/node/util/index",
		//"node:sys": "@cloudflare/unenv-preset/runtime/node/util/index",
		//
		//// define aliases for hybrid modules
		//...Object.fromEntries(
		//	hybridNodeCompatModules.flatMap((m) => [
		//		[m, `@cloudflare/unenv-preset/runtime/node/${m}/index`],
		//		[`node:${m}`, `@cloudflare/unenv-preset/runtime/node/${m}/index`],
		//	])
		//),
		//
		//// TODO: this is a hotfix and breaks unenv/fetch
		//// https://github.com/unjs/unenv/issues/364
		//"unenv/runtime/node/stream/index": "node:stream",
	},
	inject: {
		//// workerd already defines `global` and `Buffer`
		//// override the previous presets so that we use the native implementation
		//Buffer: false,
		//global: false,
		//console: "@cloudflare/unenv-preset/runtime/node/console/index",
		//process: "@cloudflare/unenv-preset/runtime/node/process/index",
	},
	polyfill: [],
	external: nodeCompatModules.flatMap((p) => [p, `node:${p}`]),
};
