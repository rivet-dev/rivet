/**
 * Provides registry software packages for tests.
 *
 * Each registry package exports a descriptor with a `commandDir` getter
 * that resolves to the package's wasm/ directory. Pass these directly
 * to AgentOs.create({ software: [...] }).
 *
 * Requires: `cd ~/agent-os-registry && make copy-wasm && make build`
 */

import { existsSync } from "node:fs";
import coreutils from "@rivet-dev/agent-os-coreutils";
import sed from "@rivet-dev/agent-os-sed";
import grep from "@rivet-dev/agent-os-grep";
import gawk from "@rivet-dev/agent-os-gawk";
import findutils from "@rivet-dev/agent-os-findutils";
import diffutils from "@rivet-dev/agent-os-diffutils";
import tar from "@rivet-dev/agent-os-tar";
import gzip from "@rivet-dev/agent-os-gzip";
import jq from "@rivet-dev/agent-os-jq";
import ripgrep from "@rivet-dev/agent-os-ripgrep";
import fd from "@rivet-dev/agent-os-fd";
import tree from "@rivet-dev/agent-os-tree";
import file from "@rivet-dev/agent-os-file";
import yq from "@rivet-dev/agent-os-yq";
import codex from "@rivet-dev/agent-os-codex";
import curl from "@rivet-dev/agent-os-curl";

/** All standard registry software packages. */
export const REGISTRY_SOFTWARE = [
	coreutils,
	sed,
	grep,
	gawk,
	findutils,
	diffutils,
	tar,
	gzip,
	jq,
	ripgrep,
	fd,
	tree,
	file,
	yq,
	codex,
	curl,
];

/** True if registry wasm binaries are available (coreutils/wasm/ exists). */
export const hasRegistryCommands = existsSync(coreutils.commandDir);

/** Skip reason for tests that need registry commands. */
export const registrySkipReason = hasRegistryCommands
	? false
	: "Registry WASM binaries not available (run: cd ~/agent-os-registry && make copy-wasm && make build)";
