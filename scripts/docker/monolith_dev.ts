#!/usr/bin/env -S deno run -A

import $ from "dax";
import { parseArgs } from "@std/cli";

const args = parseArgs(Deno.args, {
	boolean: ["no-clean", "no-build"],
});

if (!args["no-clean"]) {
	console.log("Cleaning Rivet data");
	await Deno.remove("/tmp/rivet-data", { recursive: true }).catch(() => {});
}

if (!args["no-build"]) {
	console.log("Building Rivet");
	await $`docker build -f docker/universal/Dockerfile --target monolith -t rivet .`;
}

console.log("Starting Rivet");
await $`docker run --platform linux/amd64 --name rivet --rm -v "/tmp/rivet-data:/data" -p 8080:8080 -p 9000:9000 -p 7080:7080 -p 7443:7443 -p 7500-7599:7500-7599 -p 7600-7699:7600-7699 -p 9980:9980 -e S6_VERBOSITY=3 -e S6_LOGGING=0 rivet`;

