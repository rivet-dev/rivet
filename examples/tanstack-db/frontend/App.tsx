import { useLiveQuery } from "@tanstack/react-db";
import { useState } from "react";
import { todoCollection, type Todo } from "./collection.ts";

type FilterMode = "all" | "active" | "completed";

export default function App() {
	const [filter, setFilter] = useState<FilterMode>("all");
	const [input, setInput] = useState("");

	// Live query with filtering.
	// TanStack DB's differential dataflow re-evaluates only affected rows —
	// sub-millisecond even with large collections.
	const { data: todos } = useLiveQuery(
		(q) => {
			const base = q
				.from({ todo: todoCollection })
				.orderBy(({ todo }) => todo.created_at, "desc");
			if (filter === "active") {
				return base.where(({ todo }) => (todo.completed as number) === 0);
			}
			if (filter === "completed") {
				return base.where(({ todo }) => (todo.completed as number) === 1);
			}
			return base;
		},
		[filter],
	);

	// Counts for the filter tabs (separate query so they always reflect all todos)
	const { data: allTodos } = useLiveQuery((q) =>
		q.from({ todo: todoCollection }),
	);
	const activeCount =
		allTodos?.filter((t) => (t.completed as number) === 0).length ?? 0;
	const completedCount =
		allTodos?.filter((t) => (t.completed as number) === 1).length ?? 0;

	const isReady = todoCollection.status === "ready";

	function handleAdd(e: React.FormEvent) {
		e.preventDefault();
		const title = input.trim();
		if (!title) return;
		setInput("");

		// Optimistic insert: TanStack DB updates the UI instantly.
		// The onInsert handler syncs to the actor asynchronously.
		// When the actor broadcasts the confirmed change back, the synced state
		// reconciles with the optimistic state (same UUID = confirmed).
		todoCollection.insert({
			id: crypto.randomUUID(),
			title,
			completed: 0,
			created_at: Date.now(),
		});
	}

	function handleToggle(todo: Todo) {
		todoCollection.update(todo.id, (draft) => {
			draft.completed = (draft.completed as number) === 0 ? 1 : 0;
		});
	}

	function handleDelete(id: string) {
		todoCollection.delete(id);
	}

	return (
		<div className="min-h-screen bg-black text-white font-sans">
			<div className="max-w-[560px] mx-auto px-4 py-12">
				{/* Header */}
				<header className="mb-7">
					<div className="flex items-center gap-3 mb-2">
						<h1 className="text-2xl font-bold m-0">RivetKit × TanStack DB</h1>
						<span
							className={[
								"text-xs px-2.5 py-0.5 rounded-full font-medium",
								isReady
									? "bg-success/15 text-success"
									: "bg-rivet/15 text-rivet",
							].join(" ")}
						>
							{isReady ? "● live" : "○ connecting…"}
						</span>
					</div>
					<p className="text-muted text-sm leading-relaxed m-0">
						Real-time collaborative todos. SQLite-backed via Rivet Actor,
						reactive queries via TanStack DB. Open multiple tabs to see live
						sync.
					</p>
				</header>

				{/* Add form */}
				<form onSubmit={handleAdd} className="flex gap-2 mb-4">
					<input
						className="flex-1 px-4 py-3 bg-border border border-input rounded-lg text-white text-[15px] outline-none placeholder:text-faint focus:border-rivet"
						type="text"
						value={input}
						onChange={(e) => setInput(e.target.value)}
						placeholder="Add a todo…"
						disabled={!isReady}
					/>
					<button
						className={[
							"px-5 py-3 bg-rivet text-white border-none rounded-lg text-sm font-semibold cursor-pointer",
							!isReady || !input.trim() ? "opacity-50 cursor-not-allowed" : "",
						].join(" ")}
						type="submit"
						disabled={!isReady || !input.trim()}
					>
						Add
					</button>
				</form>

				{/* Filter tabs */}
				<div className="flex gap-1.5 mb-5">
					{(["all", "active", "completed"] as FilterMode[]).map((mode) => (
						<button
							key={mode}
							className={[
								"px-3.5 py-1 border rounded-full text-[13px] cursor-pointer font-medium",
								filter === mode
									? "bg-rivet text-white border-rivet"
									: "bg-transparent text-muted border-border",
							].join(" ")}
							onClick={() => setFilter(mode)}
						>
							{mode === "all"
								? `All (${allTodos?.length ?? 0})`
								: mode === "active"
									? `Active (${activeCount})`
									: `Completed (${completedCount})`}
						</button>
					))}
				</div>

				{/* Content */}
				{!isReady ? (
					<div className="flex flex-col items-center gap-3 py-10 text-muted text-[15px]">
						<div
							className="w-6 h-6 border-2 border-border border-t-rivet rounded-full"
							style={{ animation: "spin 0.8s linear infinite" }}
						/>
						<p className="m-0">Connecting to actor…</p>
					</div>
				) : todos?.length === 0 ? (
					<p className="text-center text-faint py-10 text-[15px] m-0">
						{filter === "all"
							? "No todos yet. Add one above!"
							: `No ${filter} todos.`}
					</p>
				) : (
					<ul className="list-none p-0 m-0 bg-surface rounded-lg border border-border">
						{todos?.map((todo, i) => (
							<li
								key={todo.id}
								className={[
									"flex items-center gap-3 px-4 py-3",
									i < (todos?.length ?? 0) - 1 ? "border-b border-border" : "",
								].join(" ")}
							>
								<button
									className={[
										"w-[22px] h-[22px] border-2 rounded-[6px] text-[13px] font-bold text-white flex items-center justify-center shrink-0 cursor-pointer",
										(todo.completed as number)
											? "bg-rivet border-rivet"
											: "bg-transparent border-input",
									].join(" ")}
									onClick={() => handleToggle(todo)}
									title={
										(todo.completed as number)
											? "Mark incomplete"
											: "Mark complete"
									}
								>
									{(todo.completed as number) ? "✓" : ""}
								</button>
								<span
									className={[
										"flex-1 text-[15px]",
										(todo.completed as number)
											? "line-through text-faint"
											: "text-white",
									].join(" ")}
								>
									{todo.title}
								</span>
								<button
									className="bg-transparent border-none text-faint cursor-pointer text-sm px-1 py-0.5 rounded hover:text-white"
									onClick={() => handleDelete(todo.id)}
									title="Delete"
								>
									✕
								</button>
							</li>
						))}
					</ul>
				)}

				{/* Footer */}
				<footer className="mt-6 text-xs text-faint border-t border-border pt-4">
					Actor: <code>todoList/default</code> · Storage: SQLite (rivetkit/db) ·
					Sync: TanStack DB · Collection: @rivetkit/tanstack-db-collection
				</footer>
			</div>
		</div>
	);
}
