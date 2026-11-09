import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

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
  db: db({
    onMigrate: async (db) => {
      await db.execute(
        "CREATE TABLE IF NOT EXISTS results (task_id TEXT PRIMARY KEY, output TEXT NOT NULL)",
      );
    },
  }),
  actions: {
    // Fan-out: distribute work in parallel
    startJob: async (c, tasks: Task[]) => {
      const client = c.client<typeof registry>();
      await Promise.all(
        tasks.map(task => client.worker.getOrCreate(task.id).process(task))
      );
    },
    // Fan-in: collect results
    reportResult: async (c, result: Result) => {
      await c.db.execute(
        "INSERT OR REPLACE INTO results (task_id, output) VALUES (?, ?)",
        result.taskId,
        result.output,
      );
    },
    getResults: async (c) => {
      const rows = (await c.db.execute("SELECT task_id, output FROM results")) as {
        task_id: string;
        output: string;
      }[];
      return rows.map((row): Result => ({ taskId: row.task_id, output: row.output }));
    },
  },
});

const worker = actor({
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
