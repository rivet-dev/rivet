import { actor, setup, event } from "rivetkit";

export type Employee = {
	id: string;
	name: string;
	role: string;
	created_at: number;
};

export type ProjectStatus = "planning" | "active" | "done";

export type Project = {
	id: string;
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

export type CompanyDatabaseState = {
	company_name: string;
	employees: Employee[];
	projects: Project[];
	created_at: number;
	updated_at: number;
};

const createId = (prefix: string) =>
	`${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;

const getCompanyName = (key: unknown) => {
	if (typeof key === "string" && key.trim()) {
		return key.trim();
	}

	return "Unknown Company";
};

export const companyDatabase = actor({
	// Persistent state is isolated per company. https://rivet.dev/docs/actors/state
	createState: (c): CompanyDatabaseState => {
		const now = Date.now();
		return {
			company_name: getCompanyName(c.key[0]),
			employees: [],
			projects: [],
			created_at: now,
			updated_at: now,
		};
	},
	events: {
		employeeAdded: event<Employee>(),
		projectAdded: event<Project>(),
	},

	// Callable functions from clients. https://rivet.dev/docs/actors/actions
	actions: {
		addEmployee: (c, name: string, role: string) => {
			const employee: Employee = {
				id: createId("emp"),
				name: name.trim() || "New Employee",
				role: role.trim() || "Generalist",
				created_at: Date.now(),
			};

			c.state.employees.push(employee);
			c.state.updated_at = Date.now();
			c.broadcast("employeeAdded", employee);
			return employee;
		},

		listEmployees: (c) => c.state.employees,

		addProject: (c, name: string, status: ProjectStatus) => {
			const project: Project = {
				id: createId("proj"),
				name: name.trim() || "New Project",
				status,
				created_at: Date.now(),
			};

			c.state.projects.push(project);
			c.state.updated_at = Date.now();
			c.broadcast("projectAdded", project);
			return project;
		},

		listProjects: (c) => c.state.projects,

		getStats: (c): CompanyStats => ({
			employee_count: c.state.employees.length,
			project_count: c.state.projects.length,
			created_at: c.state.created_at,
			updated_at: c.state.updated_at,
		}),
	},
});

// Register actors for use. https://rivet.dev/docs/setup
export const registry = setup({
	use: { companyDatabase },
});
