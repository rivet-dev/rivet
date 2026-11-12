import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export interface EmployeeInput {
	employeeId: string;
	name: string;
	email: string;
	position: string;
	companyId: string;
}

export type EmployeeProfile = {
	employeeId: string;
	name: string;
	email: string;
	position: string;
	companyId: string;
	hiredAt: number;
};

export interface CompanyInput {
	name: string;
	industry: string;
}

export type CompanyProfile = {
	id: string;
	name: string;
	industry: string;
	foundedAt: number;
};

export const employee = actor({
	// The profile is saved in the actor's SQLite database: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS profile (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					employee_id TEXT NOT NULL,
					name TEXT NOT NULL,
					email TEXT NOT NULL,
					position TEXT NOT NULL,
					company_id TEXT NOT NULL,
					hired_at INTEGER NOT NULL
				)
			`);
		},
	}),

	// Save the profile from the creation input once, when the actor is created
	onCreate: async (c, input: EmployeeInput) => {
		await c.db.execute(
			"INSERT INTO profile (id, employee_id, name, email, position, company_id, hired_at) VALUES (1, ?, ?, ?, ?, ?, ?)",
			input.employeeId,
			input.name,
			input.email,
			input.position,
			input.companyId,
			Date.now(),
		);
	},

	actions: {
		// Get employee profile
		getProfile: async (c): Promise<EmployeeProfile> => {
			const [profile] = await c.db.execute<EmployeeProfile>(
				"SELECT employee_id AS employeeId, name, email, position, company_id AS companyId, hired_at AS hiredAt FROM profile WHERE id = 1",
			);
			return profile;
		},

		// Update only the fields that were passed, in a single statement
		updateProfile: async (
			c,
			updates: Partial<Omit<EmployeeInput, "companyId" | "employeeId">>,
		) => {
			const [profile] = await c.db.execute<EmployeeProfile>(
				"UPDATE profile SET name = COALESCE(?, name), email = COALESCE(?, email), position = COALESCE(?, position) WHERE id = 1 RETURNING employee_id AS employeeId, name, email, position, company_id AS companyId, hired_at AS hiredAt",
				updates.name || null,
				updates.email || null,
				updates.position || null,
			);
			return profile;
		},
	},
});

export const company = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS profile (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					company_id TEXT NOT NULL,
					name TEXT NOT NULL,
					industry TEXT NOT NULL,
					founded_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS employees (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					email TEXT NOT NULL
				)
			`);
		},
	}),

	// Save the profile from the creation input: https://rivet.dev/docs/actors/input
	onCreate: async (c, input: CompanyInput) => {
		await c.db.execute(
			"INSERT INTO profile (id, company_id, name, industry, founded_at) VALUES (1, ?, ?, ?, ?)",
			crypto.randomUUID(),
			input.name,
			input.industry,
			Date.now(),
		);
	},

	actions: {
		// Fully type-safe profile retrieval: https://rivet.dev/docs/actors/actions
		getProfile: async (c): Promise<CompanyProfile> => {
			const [profile] = await c.db.execute<CompanyProfile>(
				"SELECT company_id AS id, name, industry, founded_at AS foundedAt FROM profile WHERE id = 1",
			);
			return profile;
		},

		// Update only the fields that were passed, in a single statement
		updateProfile: async (c, updates: Partial<CompanyInput>) => {
			const [profile] = await c.db.execute<CompanyProfile>(
				"UPDATE profile SET name = COALESCE(?, name), industry = COALESCE(?, industry) WHERE id = 1 RETURNING company_id AS id, name, industry, founded_at AS foundedAt",
				updates.name || null,
				updates.industry || null,
			);
			return profile;
		},

		// Create an employee actor and track it
		createEmployee: async (
			c,
			employeeData: { name: string; email: string; position: string },
		): Promise<EmployeeProfile> => {
			const client = c.client<typeof registry>();
			const [{ companyId }] = await c.db.execute<{ companyId: string }>(
				"SELECT company_id AS companyId FROM profile WHERE id = 1",
			);

			// Generate a unique employee ID
			const employeeId = crypto.randomUUID();

			// Create employee actor using their email as the key
			await client.employee.create([employeeData.email], {
				input: {
					employeeId,
					name: employeeData.name,
					email: employeeData.email,
					position: employeeData.position,
					companyId,
				},
			});
			await c.db.execute(
				"INSERT INTO employees (email) VALUES (?)",
				employeeData.email,
			);

			// Return the employee profile
			return {
				employeeId,
				name: employeeData.name,
				email: employeeData.email,
				position: employeeData.position,
				companyId,
				hiredAt: Date.now(),
			};
		},

		// Get all employee emails
		getEmployees: async (c) => {
			const rows = await c.db.execute<{ email: string }>(
				"SELECT email FROM employees ORDER BY seq",
			);
			return rows.map((row) => row.email);
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { employee, company },
});

// Start the server on port 6420
registry.start();
