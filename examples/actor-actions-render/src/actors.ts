import { actor, setup } from "rivetkit";

export interface EmployeeInput {
	employeeId: string;
	name: string;
	email: string;
	position: string;
	companyId: string;
}

export interface EmployeeState {
	profile: EmployeeProfile;
}

export interface EmployeeProfile {
	employeeId: string;
	name: string;
	email: string;
	position: string;
	companyId: string;
	hiredAt: number;
}

export interface CompanyInput {
	name: string;
	industry: string;
}

export interface CompanyState {
	profile: CompanyProfile;
	employeeEmails: string[];
}

export interface CompanyProfile {
	id: string;
	name: string;
	industry: string;
	foundedAt: number;
}

export const employee = actor({
	createState: (_c, input: EmployeeInput): EmployeeState => ({
		profile: {
			employeeId: input.employeeId,
			name: input.name,
			email: input.email,
			position: input.position,
			companyId: input.companyId,
			hiredAt: Date.now(),
		},
	}),

	actions: {
		getProfile: (c): EmployeeProfile => c.state.profile,

		updateProfile: (
			c,
			updates: Partial<Omit<EmployeeInput, "companyId" | "employeeId">>,
		) => {
			if (updates.name) c.state.profile.name = updates.name;
			if (updates.email) c.state.profile.email = updates.email;
			if (updates.position) c.state.profile.position = updates.position;
			return c.state.profile;
		},
	},
});

export const company = actor({
	createState: (_c, input: CompanyInput): CompanyState => ({
		profile: {
			id: crypto.randomUUID(),
			name: input.name,
			industry: input.industry,
			foundedAt: Date.now(),
		},
		employeeEmails: [],
	}),

	actions: {
		getProfile: (c): CompanyProfile => c.state.profile,

		updateProfile: (c, updates: Partial<CompanyInput>) => {
			if (updates.name) c.state.profile.name = updates.name;
			if (updates.industry) c.state.profile.industry = updates.industry;
			return c.state.profile;
		},

		createEmployee: async (
			c,
			employeeData: { name: string; email: string; position: string },
		): Promise<EmployeeProfile> => {
			const client = c.client<typeof registry>();
			const employeeId = crypto.randomUUID();

			await client.employee.create([employeeData.email], {
				input: {
					employeeId,
					name: employeeData.name,
					email: employeeData.email,
					position: employeeData.position,
					companyId: c.state.profile.id,
				},
			});
			c.state.employeeEmails.push(employeeData.email);

			return {
				employeeId,
				name: employeeData.name,
				email: employeeData.email,
				position: employeeData.position,
				companyId: c.state.profile.id,
				hiredAt: Date.now(),
			};
		},

		getEmployees: (c) => c.state.employeeEmails,
	},
});

export const registry = setup({
	use: { employee, company },
});
