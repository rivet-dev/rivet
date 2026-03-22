#!/usr/bin/env bun
/**
 * Rivet Cloud CLI — entry point.
 *
 * Usage:
 *   rivet-cloud deploy [options]
 *   rivet-cloud logs [options]
 *   rivet-cloud --help
 *   rivet-cloud --version
 */

import { Command } from "commander";
import { registerDeployCommand } from "./commands/deploy.ts";
import { registerLogsCommand } from "./commands/logs.ts";
import { colors } from "./utils/output.ts";

const pkg = await Bun.file(new URL("../package.json", import.meta.url)).json();

const program = new Command();

program
	.name("rivet-cloud")
	.description(
		[
			colors.accentBold("Rivet Cloud CLI"),
			colors.secondary(
				"Deploy Docker images and stream logs for Rivet Cloud managed pools.",
			),
			"",
			colors.dim("Docs: https://rivet.dev/docs"),
			colors.dim("Dashboard: https://hub.rivet.dev"),
		].join("\n"),
	)
	.version(pkg.version, "-V, --version", "Output the current version")
	.helpOption("-h, --help", "Display help for command");

registerDeployCommand(program);
registerLogsCommand(program);

// Show help when no command is provided
if (process.argv.length <= 2) {
	program.help();
}

program.parseAsync(process.argv);
