import { createClient } from "rivetkit/client";
import type { registry } from "../src/registry.js";

// Get endpoint from environment variable or default to localhost
const endpoint = process.env.RIVETKIT_ENDPOINT ?? "http://localhost:8080";
console.log("🔗 Using endpoint:", endpoint);

// Create RivetKit client
const client = createClient<typeof registry>(endpoint);

async function main() {
	console.log("🚀 SQLite Raw Database Demo");

	try {
		// Create todo list instance
		const todoList = client.todoList.getOrCreate("my-todos");

		// Add some todos
		console.log("\n📝 Adding todos...");
		const todo1 = await todoList.addTodo("Buy groceries");
		console.log("Added:", todo1);

		const todo2 = await todoList.addTodo("Write documentation");
		console.log("Added:", todo2);

		const todo3 = await todoList.addTodo("Review pull requests");
		console.log("Added:", todo3);

		// Get all todos
		console.log("\n📋 Getting all todos...");
		const todos = await todoList.getTodos();
		console.log("Todos:", todos);

		// Toggle a todo (assuming first todo has id 1)
		console.log("\n✅ Toggling first todo...");
		const toggled = await todoList.toggleTodo(1);
		console.log("Toggled:", toggled);

		// Get todos again to see the change
		console.log("\n📋 Getting all todos after toggle...");
		const todosAfterToggle = await todoList.getTodos();
		console.log("Todos:", todosAfterToggle);

		// Delete a todo (assuming second todo has id 2)
		console.log("\n🗑️ Deleting second todo...");
		const deleted = await todoList.deleteTodo(2);
		console.log("Deleted:", deleted);

		// Get todos one more time
		console.log("\n📋 Final todos list...");
		const finalTodos = await todoList.getTodos();
		console.log("Todos:", finalTodos);

		console.log("\n✅ Demo completed!");
	} catch (error) {
		console.error("❌ Error:", error);
		process.exit(1);
	}
}

main().catch(console.error);
