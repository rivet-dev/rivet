/**
 * Output helpers — Rivet brand colors and formatted messages.
 *
 * Rivet brand colors (from tailwind.config.mjs):
 *   accent   #FF4500  (orange-red)
 *   text-primary  #FAFAFA
 *   text-secondary #A0A0A0
 *   background    #000000
 */

import chalk from "chalk";

export const colors = {
	/** Rivet brand accent — #FF4500 orange-red */
	accent: chalk.hex("#FF4500"),
	/** Rivet brand accent, bold */
	accentBold: chalk.hex("#FF4500").bold,
	/** Bright primary text */
	primary: chalk.hex("#FAFAFA"),
	/** Secondary / muted text */
	secondary: chalk.hex("#A0A0A0"),
	/** Alias for secondary */
	dim: chalk.hex("#A0A0A0"),
	/** Success green */
	success: chalk.hex("#4ade80"),
	/** Warning yellow */
	warning: chalk.hex("#facc15"),
	/** Error red */
	error: chalk.hex("#f87171").bold,
	/** Bold white label */
	label: chalk.white.bold,
	/** Monospace code */
	code: chalk.cyan,
};

/** Print the Rivet logo / wordmark prefix. */
export function logo(): string {
	return colors.accentBold("▶ rivet");
}

/** Print a section header line. */
export function header(text: string): void {
	console.log(`\n${colors.accentBold("◆")} ${colors.label(text)}`);
}

/** Print a success line with a checkmark. */
export function success(text: string): void {
	console.log(`  ${colors.success("✓")} ${text}`);
}

/** Print a detail / sub-item line. */
export function detail(key: string, value: string): void {
	console.log(`  ${colors.dim(key + ":")} ${colors.primary(value)}`);
}

/** Print an info line. */
export function info(text: string): void {
	console.log(`  ${colors.secondary("·")} ${colors.secondary(text)}`);
}

/** Print an error and exit. */
export function fatal(text: string, cause?: unknown): never {
	console.error(`\n${colors.error("✗ Error:")} ${text}`);
	if (cause) {
		const causeMsg = cause instanceof Error ? cause.message : String(cause);
		console.error(`  ${colors.dim(causeMsg)}`);
	}
	process.exit(1);
}

/** Create an error with a cause chain. */
export function error(message: string, cause: unknown): Error {
	const error = new Error(message);
	error.cause = cause;
	return error;
}

/** Format a log timestamp for display. */
export function formatTimestamp(iso: string): string {
	try {
		const d = new Date(iso);
		const hh = String(d.getUTCHours()).padStart(2, "0");
		const mm = String(d.getUTCMinutes()).padStart(2, "0");
		const ss = String(d.getUTCSeconds()).padStart(2, "0");
		return colors.dim(`${hh}:${mm}:${ss}`);
	} catch {
		return colors.dim(iso);
	}
}

/** Format a region slug for display. */
export function formatRegion(region: string): string {
	return colors.secondary(`[${region}]`);
}
