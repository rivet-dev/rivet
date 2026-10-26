import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export type Employee = {
	id: number;
	name: string;
	role: string;
	created_at: number;
};

export type ProjectStatus = "planning" | "active" | "done";

export type Project = {
	id: number;
	name: string;
	status: ProjectStatus;
	created_at: number;
};

export type CompanyStats = {
	employee_count: number;
	project_count: number;
	created_at: number;
	updated_at: number;
};

// docs:start actor
export const companyDatabase = actor({
	// docs:end actor
	// docs:start schema
	// Every tenant's Actor has its own SQLite database: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS company (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					name TEXT NOT NULL,
					created_at INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS employees (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					role TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS projects (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					status TEXT NOT NULL CHECK (status IN ('planning', 'active', 'done')),
					created_at INTEGER NOT NULL
				)
			`);
		},
	}),
	// docs:end schema

	// docs:start actor
	// Runs once, the first time this tenant's Actor is created: https://rivet.dev/docs/actors/lifecycle
	onCreate: async (c) => {
		const now = Date.now();
		await c.db.execute(
			"INSERT INTO company (id, name, created_at, updated_at) VALUES (1, ?, ?, ?)",
			getCompanyName(c.key[0]),
			now,
			now,
		);
	},
	// docs:end actor

	// docs:start broadcast
	events: {
		employeeAdded: event<Employee>(),
		projectAdded: event<Project>(),
	},
	// docs:end broadcast

	// Callable functions from clients: https://rivet.dev/docs/actors/actions
	actions: {
		// docs:start read
		listEmployees: (c) =>
			c.db.execute<Employee>(
				"SELECT id, name, role, created_at FROM employees ORDER BY id",
			),

		listProjects: (c) =>
			c.db.execute<Project>(
				"SELECT id, name, status, created_at FROM projects ORDER BY id",
			),

		getStats: async (c) => {
			const [stats] = await c.db.execute<CompanyStats>(`
				SELECT
					(SELECT COUNT(*) FROM employees) AS employee_count,
					(SELECT COUNT(*) FROM projects) AS project_count,
					created_at,
					updated_at
				FROM company
			`);
			return stats;
		},
		// docs:end read

		// docs:start write
		addEmployee: async (c, name: string, role: string) => {
			const now = Date.now();
			// Insert the row and bump updated_at together, or not at all
			const employee = await c.db.transaction(
				async (tx) => {
					const [row] = await tx.execute<Employee>(
						"INSERT INTO employees (name, role, created_at) VALUES (?, ?, ?) RETURNING id, name, role, created_at",
						name.trim() || "New Employee",
						role.trim() || "Generalist",
						now,
					);
					await tx.execute("UPDATE company SET updated_at = ?", now);
					return row;
				},
				{ name: "add-employee" },
			);
			// docs:start broadcast
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("employeeAdded", employee);
			// docs:end broadcast
			return employee;
		},

		addProject: async (c, name: string, status: ProjectStatus) => {
			const now = Date.now();
			const project = await c.db.transaction(
				async (tx) => {
					const [row] = await tx.execute<Project>(
						"INSERT INTO projects (name, status, created_at) VALUES (?, ?, ?) RETURNING id, name, status, created_at",
						name.trim() || "New Project",
						status,
						now,
					);
					await tx.execute("UPDATE company SET updated_at = ?", now);
					return row;
				},
				{ name: "add-project" },
			);
			// docs:start broadcast
			c.broadcast("projectAdded", project);
			// docs:end broadcast
			return project;
		},
		// docs:end write
	},
	// docs:start actor
});
// docs:end actor

// docs:start registry
// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { companyDatabase },
});

registry.start();
// docs:end registry

const getCompanyName = (key: unknown) => {
	if (typeof key === "string" && key.trim()) {
		return key.trim();
	}

	return "Unknown Company";
};
