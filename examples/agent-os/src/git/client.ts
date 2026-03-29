// Clone a local repository and check out a feature branch in the clone.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

type ExecResult = {
	stdout: string;
	stderr: string;
	exitCode: number;
};

function parseCurrentBranch(output: string): string {
	const branch = output
		.split("\n")
		.map((line) => line.trim())
		.find((line) => line.startsWith("* "))
		?.slice(2)
		.trim();

	if (!branch) {
		throw new Error(`could not determine current branch from:\n${output}`);
	}

	return branch;
}

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

async function run(command: string): Promise<ExecResult> {
	const result = (await agent.exec(command)) as ExecResult;
	if (result.exitCode !== 0) {
		throw new Error(
			`command failed: ${command}\n${result.stderr || result.stdout}`,
		);
	}
	return result;
}

await run("git init /tmp/origin");
await agent.writeFile("/tmp/origin/README.md", "# demo repo\n");
await run("git -C /tmp/origin add README.md");
await run("git -C /tmp/origin commit -m 'initial commit'");

const defaultBranch = parseCurrentBranch(
	(await run("git -C /tmp/origin branch")).stdout,
);

await run("git -C /tmp/origin checkout -b feature");
await agent.writeFile("/tmp/origin/feature.txt", "checked out from feature\n");
await run("git -C /tmp/origin add feature.txt");
await run("git -C /tmp/origin commit -m 'add feature file'");
await run(`git -C /tmp/origin checkout ${defaultBranch}`);

await run("git clone /tmp/origin /tmp/clone");
console.log("clone branches before checkout:");
console.log((await run("git -C /tmp/clone branch")).stdout.trim());

await run("git -C /tmp/clone checkout feature");
console.log("clone branches after checkout:");
console.log((await run("git -C /tmp/clone branch")).stdout.trim());

const featureFile = (await agent.readFile("/tmp/clone/feature.txt")) as Uint8Array;
console.log("feature.txt:", new TextDecoder().decode(featureFile).trim());
