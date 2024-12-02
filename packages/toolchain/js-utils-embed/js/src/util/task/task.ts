import { parseArgs } from "@std/cli/parse-args";
import { printError, UserError } from "./error.ts";
import { z } from "zod";

export interface Task<T extends z.ZodType> {
	inputSchema: T;
	run(input: z.infer<T>): Promise<void>;
}

export async function runTask<T extends z.ZodType>(task: Task<T>) {
	Deno.addSignalListener(Deno.build.os == "windows" ? "SIGBREAK" : "SIGINT", async () => {
		console.log("Received shutdown signal");
		Deno.exit(0);
	});

	let exitCode = 0;
	try {
		// Parse flags
		const args = parseArgs(Deno.args);
		const inputJson = args["input"];
		if (!inputJson) {
			throw new UserError("Missing --input argument");
		}

		// Parse input
		let input;
		try {
			input = JSON.parse(inputJson);
		} catch (cause) {
			throw new UserError("Invalid input JSON", { cause });
		}

		// Validate input using the task's inputSchema
		const validatedInput = task.inputSchema.safeParse(input);
		if (!validatedInput.success) {
			throw new UserError("Input violates schema", { details: JSON.stringify(validatedInput.error, null, 2) });
		}

		// Execute task
		await task.run(validatedInput.data);
	} catch (err) {
		printError(err);
		exitCode = 1;
	}

	Deno.exit(exitCode);
}
