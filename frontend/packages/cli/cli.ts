#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import os from "node:os";

const platform = os.platform();

function computeTargetFilename() {
	if (platform === "win32") {
		return "rivet-cli.exe";
	}

	return "rivet-cli";
}

execFileSync(join(__dirname, computeTargetFilename()), process.argv.slice(2), {
	stdio: "inherit",
});
