import { randomUUID } from "node:crypto";
import { createClient } from "rivetkit/client";
import type { ResearchProgress, registry } from "../src/actors.ts";

const question = process.argv.slice(2).join(" ").trim();
if (!question) {
	console.error('Usage: pnpm client "<your research question>"');
	process.exit(1);
}

const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
});
const coordinator = client.researchCoordinator.getOrCreate([randomUUID()]).connect();

const finished = new Promise<void>((resolve) => {
	coordinator.on("progress", ({ status, message }: ResearchProgress) => {
		console.log(`[${status}] ${message}`);
		if (status === "done" || status === "failed") {
			resolve();
		}
	});
});

await coordinator.start(question);
await finished;

const result = await coordinator.getResult();
console.log(`\n${result.report ?? `Research failed: ${result.error}`}`);
console.log(`\nFindings: ${result.findings.length}, list-price cost: $${result.costUsd.toFixed(2)}`);

await coordinator.dispose();
process.exit(result.status === "done" ? 0 : 1);
