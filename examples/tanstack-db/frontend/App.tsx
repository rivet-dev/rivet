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
		<div style={s.page}>
			<div style={s.container}>
				<header style={s.header}>
					<div style={s.headerTop}>
						<h1 style={s.title}>RivetKit × TanStack DB</h1>
						<span
							style={{ ...s.badge, ...(isReady ? s.badgeOn : s.badgeOff) }}
						>
							{isReady ? "● live" : "○ connecting…"}
						</span>
					</div>
					<p style={s.subtitle}>
						Real-time collaborative todos. SQLite-backed via Rivet Actor, reactive
						queries via TanStack DB. Open multiple tabs to see live sync.
					</p>
				</header>

				<form onSubmit={handleAdd} style={s.form}>
					<input
						style={s.input}
						type="text"
						value={input}
						onChange={(e) => setInput(e.target.value)}
						placeholder="Add a todo…"
						disabled={!isReady}
					/>
					<button
						style={{
							...s.addBtn,
							...(!isReady || !input.trim() ? s.btnDisabled : {}),
						}}
						type="submit"
						disabled={!isReady || !input.trim()}
					>
						Add
					</button>
				</form>

				<div style={s.filters}>
					{(["all", "active", "completed"] as FilterMode[]).map((mode) => (
						<button
							key={mode}
							style={{
								...s.filterBtn,
								...(filter === mode ? s.filterBtnActive : {}),
							}}
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

				{!isReady ? (
					<div style={s.empty}>
						<div style={s.spinner} />
						<p>Connecting to actor…</p>
					</div>
				) : todos?.length === 0 ? (
					<p style={s.emptyText}>
						{filter === "all"
							? "No todos yet. Add one above!"
							: `No ${filter} todos.`}
					</p>
				) : (
					<ul style={s.list}>
						{todos?.map((todo) => (
							<li key={todo.id} style={s.item}>
								<button
									style={{
										...s.checkbox,
										...((todo.completed as number) ? s.checkboxDone : {}),
									}}
									onClick={() => handleToggle(todo)}
									title={
										(todo.completed as number) ? "Mark incomplete" : "Mark complete"
									}
								>
									{(todo.completed as number) ? "✓" : ""}
								</button>
								<span
									style={{
										...s.todoText,
										...((todo.completed as number) ? s.todoTextDone : {}),
									}}
								>
									{todo.title}
								</span>
								<button
									style={s.deleteBtn}
									onClick={() => handleDelete(todo.id)}
									title="Delete"
								>
									✕
								</button>
							</li>
						))}
					</ul>
				)}

				<footer style={s.footer}>
					Actor: <code>todoList/default</code> · Storage: SQLite (rivetkit/db) ·
					Sync: TanStack DB · Collection: @rivetkit/tanstack-db-collection
				</footer>
			</div>
		</div>
	);
}

const s: Record<string, React.CSSProperties> = {
	page: {
		minHeight: "100vh",
		background: "#000",
		color: "#fff",
		fontFamily:
			"-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Inter', Roboto, sans-serif",
	},
	container: {
		maxWidth: 560,
		margin: "0 auto",
		padding: "48px 16px",
	},
	header: {
		marginBottom: 28,
	},
	headerTop: {
		display: "flex",
		alignItems: "center",
		gap: 12,
		marginBottom: 8,
	},
	title: {
		margin: 0,
		fontSize: 24,
		fontWeight: 700,
	},
	badge: {
		fontSize: 12,
		padding: "3px 10px",
		borderRadius: 99,
		fontWeight: 500,
	},
	badgeOn: { background: "rgba(48,209,88,0.15)", color: "#30d158" },
	badgeOff: { background: "rgba(255,79,0,0.15)", color: "#ff4f00" },
	subtitle: {
		margin: 0,
		color: "#8e8e93",
		fontSize: 14,
		lineHeight: 1.6,
	},
	form: {
		display: "flex",
		gap: 8,
		marginBottom: 16,
	},
	input: {
		flex: 1,
		padding: "12px 16px",
		background: "#2c2c2e",
		border: "1px solid #3a3a3c",
		borderRadius: 8,
		color: "#fff",
		fontSize: 15,
		outline: "none",
	},
	addBtn: {
		padding: "12px 20px",
		background: "#ff4f00",
		color: "#fff",
		border: "none",
		borderRadius: 8,
		fontSize: 14,
		fontWeight: 600,
		cursor: "pointer",
	},
	btnDisabled: {
		opacity: 0.5,
		cursor: "not-allowed",
	},
	filters: {
		display: "flex",
		gap: 6,
		marginBottom: 20,
	},
	filterBtn: {
		padding: "5px 14px",
		border: "1px solid #2c2c2e",
		borderRadius: 99,
		background: "transparent",
		fontSize: 13,
		cursor: "pointer",
		color: "#8e8e93",
		fontWeight: 500,
	},
	filterBtnActive: {
		background: "#ff4f00",
		color: "#fff",
		borderColor: "#ff4f00",
	},
	list: {
		listStyle: "none",
		padding: 0,
		margin: 0,
		background: "#1c1c1e",
		borderRadius: 8,
		border: "1px solid #2c2c2e",
	},
	item: {
		display: "flex",
		alignItems: "center",
		gap: 12,
		padding: "12px 16px",
		borderBottom: "1px solid #2c2c2e",
	},
	checkbox: {
		width: 22,
		height: 22,
		border: "2px solid #3a3a3c",
		borderRadius: 6,
		background: "transparent",
		cursor: "pointer",
		fontSize: 13,
		fontWeight: 700,
		color: "#fff",
		display: "flex",
		alignItems: "center",
		justifyContent: "center",
		flexShrink: 0,
	},
	checkboxDone: {
		background: "#ff4f00",
		borderColor: "#ff4f00",
	},
	todoText: {
		flex: 1,
		fontSize: 15,
		color: "#fff",
	},
	todoTextDone: {
		textDecoration: "line-through",
		color: "#6e6e73",
	},
	deleteBtn: {
		background: "transparent",
		border: "none",
		color: "#6e6e73",
		cursor: "pointer",
		fontSize: 14,
		padding: "2px 4px",
		borderRadius: 4,
	},
	empty: {
		display: "flex",
		flexDirection: "column",
		alignItems: "center",
		gap: 12,
		padding: "40px 0",
		color: "#8e8e93",
		fontSize: 15,
	},
	spinner: {
		width: 24,
		height: 24,
		border: "2px solid #2c2c2e",
		borderTop: "2px solid #ff4f00",
		borderRadius: "50%",
		animation: "spin 0.8s linear infinite",
	},
	emptyText: {
		textAlign: "center",
		color: "#6e6e73",
		padding: "40px 0",
		fontSize: 15,
		margin: 0,
	},
	footer: {
		marginTop: 24,
		fontSize: 12,
		color: "#6e6e73",
		borderTop: "1px solid #2c2c2e",
		paddingTop: 16,
	},
};
