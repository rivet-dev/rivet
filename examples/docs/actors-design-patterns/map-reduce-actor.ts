import { actor, setup } from "rivetkit";

interface Task {
  id: string;
  data: string;
}

interface Result {
  taskId: string;
  output: string;
}

// Coordinator fans out tasks, then fans in results
const coordinator = actor({
  state: { results: [] as Result[] },
  actions: {
    // Fan-out: distribute work in parallel
    startJob: async (c, tasks: Task[]) => {
      const client = c.client<typeof registry>();
      await Promise.all(
        tasks.map(task => client.worker.getOrCreate(task.id).process(task))
      );
    },
    // Fan-in: collect results
    reportResult: (c, result: Result) => {
      c.state.results.push(result);
    },
    getResults: (c) => c.state.results,
  },
});

const worker = actor({
  state: {},
  actions: {
    process: async (c, task: Task) => {
      const result = { taskId: task.id, output: `Processed ${task.data}` };
      const client = c.client<typeof registry>();
      await client.coordinator.getOrCreate("main").reportResult(result);
    },
  },
});

export const registry = setup({
  use: { coordinator, worker },
});
