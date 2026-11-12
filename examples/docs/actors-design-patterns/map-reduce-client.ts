import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface Task {
  id: string;
  data: string;
}

interface Result {
  taskId: string;
  output: string;
}

const coordinator = actor({
  vars: { results: [] as Result[] },
  actions: {
    startJob: async (c, tasks: Task[]) => {},
    reportResult: (c, result: Result) => { c.vars.results.push(result); },
    getResults: (c) => c.vars.results,
  },
});

const worker = actor({
  actions: {
    process: async (c, task: Task) => {},
  },
});

const registry = setup({ use: { coordinator, worker } });
const client = createClient<typeof registry>("http://localhost:6420");

const coordinatorHandle = client.coordinator.getOrCreate(["main"]);

// Start a job with multiple tasks
await coordinatorHandle.startJob([
  { id: "task-1", data: "..." },
  { id: "task-2", data: "..." },
  { id: "task-3", data: "..." },
]);

// Results are collected as workers report back
const results = await coordinatorHandle.getResults();
