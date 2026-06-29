import { useState, useEffect } from "react";
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./actors";

const { useActor } = createRivetKit<typeof registry>();

function TodoApp() {
  // Connect to the actor over WebSockets
  const { connection, connStatus, useEvent } = useActor({
    name: "todoList",
    key: ["my-list"],
  });
  const [todos, setTodos] = useState<
    { id: number; title: string; completed: number }[]
  >([]);
  const [input, setInput] = useState("");

  // Load initial data
  useEffect(() => {
    if (!connection) return;
    connection.getTodos().then(setTodos);
  }, [connection]);

  // Live updates from the actor
  useEvent("todosUpdated", setTodos);

  if (connStatus !== "connected") return <div>Connecting...</div>;

  return (
    <div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          connection!.addTodo(input);
          setInput("");
        }}
      >
        <input value={input} onChange={(e) => setInput(e.target.value)} />
        <button type="submit">Add</button>
      </form>
      <ul>
        {todos.map((todo) => (
          <li
            key={todo.id}
            onClick={() => connection!.toggleTodo(todo.id)}
            style={{
              textDecoration: todo.completed ? "line-through" : "none",
              cursor: "pointer",
            }}
          >
            {todo.title}
          </li>
        ))}
      </ul>
    </div>
  );
}
