import { relative } from "@std/path";
import * as colors from "@std/fmt/colors";
import { fromError as fromValidationError } from "zod-validation-error";

/**
 * Error type known to this program. If an error does not extend KnownError,
 * it's a bug.
 */
class KnownError extends Error {
}

export interface BuildErrorOpts extends ErrorOptions {
	details?: string;

	/**
	 * Path of the relevant file.
	 */
	path?: string;
	/**
	 * Paths of the relevant files.
	 */
	paths?: string[];
}

class BuildError extends KnownError {
	public readonly details?: string;
	public readonly paths: string[] = [];

	constructor(message: string, opts?: BuildErrorOpts) {
		super(message, opts);
		this.name = "BuildError";

		this.details = opts?.details;
		if (opts?.path) this.paths.push(opts.path);
		if (opts?.paths) this.paths.push(...opts.paths);
	}
}

export interface UserErrorOpts extends BuildErrorOpts {
	suggest?: string;
}

export class UserError extends BuildError {
	public readonly suggest?: string;

	constructor(message: string, opts?: UserErrorOpts) {
		super(message, opts);
		this.name = "UserError";
		this.suggest = opts?.suggest;
	}
}

export interface InternalErrorOpts extends BuildErrorOpts {
	originalError?: Error;
}

export class InternalError extends BuildError {
	public readonly originalError?: Error;

	constructor(message: string, opts?: InternalErrorOpts) {
		super(message, opts);
		this.name = "InternalError";

		this.originalError = opts?.originalError;
	}
}

export class AbortError extends InternalError {
	constructor(message: string) {
		super(message);
		this.name = "AbortError";
	}
}

export class UnreachableError extends InternalError {
	constructor(public readonly value: never) {
		super("Unreachable.");
		this.name = "UnreachableError";
	}
}

export interface CommandErrorOpts extends BuildErrorOpts {
	commandOutput: Deno.CommandOutput;
}

export class CommandError extends BuildError {
	public readonly commandOutput: Deno.CommandOutput;

	constructor(message: string, opts: CommandErrorOpts) {
		super(message, opts);
		this.name = "CommandError";

		this.commandOutput = opts?.commandOutput;
	}
}

export class CombinedError extends KnownError {
	public readonly errors: Error[] = [];

	constructor(errors: Error[]) {
		super("Combined error");
		this.name = "CombinedError";

		// Flatten errors
		for (const error of errors) {
			if (error instanceof CombinedError) {
				this.errors.push(...error.errors);
			} else {
				this.errors.push(error);
			}
		}
	}
}
interface ValidationErrorOpts extends UserErrorOpts {
	validationError: Error;
	info?: Record<string, unknown>;
}

export class ValidationError extends UserError {
	public readonly validationError: Error;
	public readonly info: Record<string, unknown> = {};
	constructor(message: string, opts: ValidationErrorOpts) {
		super(message, opts);
		this.name = "ValidationError";
		if (opts.info) {
			this.info = opts.info;
		}
		this.validationError = opts.validationError;
	}
}

export function printError(error: unknown) {
	// Padding
	console.error();

	if (error instanceof CombinedError) {
		for (const subError of error.errors) {
			printError(subError);
		}

		console.error();
		console.error(
			`${
				colors.bold(
					colors.red(
						`Failed. Found ${error.errors.length} ${error.errors.length == 1 ? "error" : "errors"}.`,
					),
				)
			}`,
		);
	} else if (error instanceof KnownError) {
		let str = "";

		// Message
		str += `${colors.bold(colors.red("error"))}: ${colors.bold(error.message)}\n`;
		str += "\n";

		if (error instanceof BuildError && error.details) {
			// Details
			for (const line of error.details.split("\n")) {
				str += `  ${colors.dim(line)}\n`;
			}
		}

		if (error instanceof ValidationError) {
			str += `  ${colors.dim(fromValidationError(error.validationError).toString())}\n`;
		}

		if (error instanceof UserError && error.suggest) {
			// Suggest
			for (const line of error.suggest.split("\n")) {
				str += `  ${colors.brightBlue(line)}\n`;
			}
		}

		if (error instanceof BuildError) {
			// Path
			if (error.paths) {
				let i = 0;
				for (const path of error.paths) {
					const pathRelative = relative(Deno.cwd(), path);
					if (i == 0) {
						str += `  ${colors.dim("see " + pathRelative)}\n`;
					} else if (i < 4) {
						str += `  ${colors.dim("see " + pathRelative)}\n`;
					} else {
						str += `  ${colors.dim(`...${error.paths.length - i} more`)}\n`;
						break;
					}
					i++;
				}
			}
		}

		if (error instanceof UnreachableError) {
			str += `  ${colors.dim("value")}: ${JSON.stringify(error.value)}\n`;
		}

		if (error instanceof InternalError) {
			// Stack
			str += prettyPrintStack(error.originalError ?? error);
		}

		if (error instanceof CommandError) {
			// Command output
			try {
				const stdout = new TextDecoder().decode(error.commandOutput.stdout).trimEnd();
				str += `  ${colors.dim("stdout")}: ${stdout}\n`;
				for (const line of stdout.split("\n")) {
					str += `  ${colors.dim(line)}\n`;
				}
			} catch (err) {
				// HACK: If the command did not pipe stdout, Deno throws a TypeError. There's no
				// way to check if the command piped stdout without catching the error.
				if (err instanceof Error && err.name !== "TypeError") throw err;
			}

			try {
				if (error.commandOutput.stderr.length > 0) {
					const stderr = new TextDecoder().decode(error.commandOutput.stderr).trimEnd();
					str += `  ${colors.dim("stderr")}: ${stderr}\n`;
					for (const line of stderr.split("\n")) {
						str += `  ${colors.dim(line)}\n`;
					}
				}
			} catch (err) {
				// HACK: See above
				if (err instanceof Error && err.name !== "TypeError") throw err;
			}
		}

		console.error(str);
	} else if (error instanceof Error) {
		let str = `${colors.bold(colors.red("[UNCAUGHT] " + error.name))}: ${error.message}\n`;

		// Stack
		str += prettyPrintStack(error);

		console.error(str);
	} else {
		// Unknown error type

		const str = `${colors.bold(colors.red("[UNCAUGHT] " + error))}\n`;
		console.error(str);
	}
}

function prettyPrintStack(error: Error): string {
	if (!error.stack) return "";

	let str = "";
	for (let line of error.stack.split("\n")) {
		line = line.trim();
		if (line.startsWith("at ")) str += `  ${colors.dim(line)}\n`;
	}
	return str;
}
