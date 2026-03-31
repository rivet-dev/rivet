// Cron scheduling: schedule recurring commands inside the VM.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Schedule a command to run every second (for demo purposes)
const job = (await agent.scheduleCron({
	schedule: "* * * * * *",
	action: { type: "exec", command: "echo", args: ["cron tick"] },
})) as { id: string };
console.log("Scheduled cron job:", job.id);

// List all scheduled jobs
const jobs = (await agent.listCronJobs()) as Array<{
	id: string;
	schedule: string;
}>;
console.log("Active cron jobs:", jobs);

// Wait a few seconds to let the cron fire
console.log("Waiting 3 seconds for cron ticks...");
await new Promise((r) => setTimeout(r, 3000));

// Cancel the job
await agent.cancelCronJob(job.id);
console.log("Cancelled cron job:", job.id);

// Verify it's gone
const remaining = (await agent.listCronJobs()) as Array<{ id: string }>;
console.log("Remaining cron jobs:", remaining.length);
