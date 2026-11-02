import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
});

// Each key is its own todo list actor with its own Turso client.
const list = client.todoList.getOrCreate(["groceries"]);

const milk = await list.addTodo("Buy milk");
await list.addTodo("Buy eggs");
await list.toggleTodo(milk.id);
console.log("todos:", await list.listTodos());

// Destroying the actor deletes its rows from Turso and closes its client.
await list.deleteList();
console.log("deleted the groceries list");
