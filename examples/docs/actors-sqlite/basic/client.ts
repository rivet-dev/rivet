import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");
const handle = client.todoList.getOrCreate(["main"]);

await handle.addTodo("Write SQLite docs");
await handle.addTodo("Ship docs update");

const todos = await handle.getTodos();
console.log(todos);
