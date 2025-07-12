#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import os from "node:os";
import { join } from "node:path";

const platform = os.platform();

function computeTargetFilename() {
	if (platform === "win32") {
		return "rivet-cli.exe";
	}

	return "rivet-cli";
}

try {
	execFileSync(
		join(__dirname, computeTargetFilename()),
		process.argv.slice(2),
		{
			stdio: "inherit",
		},
	);
} catch (error) {
	if (error.status) {
		process.exit(error.status);
	}
	throw error;
}
