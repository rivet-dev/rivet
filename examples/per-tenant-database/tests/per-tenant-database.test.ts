import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/actors.ts";

test("Company data is isolated by actor key", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const alpha = client.companyDatabase.getOrCreate(["Alpha Co"]);
	const beta = client.companyDatabase.getOrCreate(["Beta Co"]);

	await alpha.addEmployee("Ava", "Engineering");
	await alpha.addProject("Phoenix", "active");
	await beta.addEmployee("Ben", "Sales");

	const alphaEmployees = await alpha.listEmployees();
	const betaEmployees = await beta.listEmployees();
	const alphaProjects = await alpha.listProjects();
	const betaProjects = await beta.listProjects();

	expect(alphaEmployees).toHaveLength(1);
	expect(betaEmployees).toHaveLength(1);
	expect(alphaEmployees[0].name).toBe("Ava");
	expect(betaEmployees[0].name).toBe("Ben");
	expect(alphaProjects).toHaveLength(1);
	expect(betaProjects).toHaveLength(0);
});

test("Stats reflect per-company state", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const company = client.companyDatabase.getOrCreate(["Stats Co"]);

	const initialStats = await company.getStats();
	expect(initialStats.employee_count).toBe(0);
	expect(initialStats.project_count).toBe(0);

	await company.addEmployee("Lina", "Finance");
	await company.addEmployee("Omar", "Support");
	await company.addProject("Drift", "planning");

	const updatedStats = await company.getStats();
	expect(updatedStats.employee_count).toBe(2);
	expect(updatedStats.project_count).toBe(1);
	expect(updatedStats.updated_at).toBeGreaterThanOrEqual(
		updatedStats.created_at,
	);
});
