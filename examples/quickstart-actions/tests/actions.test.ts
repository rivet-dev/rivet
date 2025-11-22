import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/backend/registry";

describe("company and employee actors", () => {
	test("create company actor with input and get profile", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company with EIN as key
		const company = await client.company.create(["12-3456789"], {
			input: {
				name: "Acme Corp",
				industry: "Technology",
			},
		});

		// Get profile
		const profile = await company.getProfile();

		expect(profile).toMatchObject({
			id: expect.any(String),
			name: "Acme Corp",
			industry: "Technology",
			foundedAt: expect.any(Number),
		});
	});

	test("update company profile", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company
		const company = await client.company.create(["23-4567890"], {
			input: {
				name: "Tech Startup",
				industry: "Software",
			},
		});

		// Update profile
		const updatedProfile = await company.updateProfile({
			name: "Tech Unicorn",
			industry: "SaaS",
		});

		expect(updatedProfile.name).toBe("Tech Unicorn");
		expect(updatedProfile.industry).toBe("SaaS");

		// Verify changes persist
		const profile = await company.getProfile();
		expect(profile.name).toBe("Tech Unicorn");
		expect(profile.industry).toBe("SaaS");
	});

	test("company creates employee actor", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company
		const company = await client.company.create(["34-5678901"], {
			input: {
				name: "Growing Corp",
				industry: "Technology",
			},
		});

		// Company creates employee
		const employeeProfile = await company.createEmployee({
			name: "Jane Smith",
			email: "jane@growingcorp.com",
			position: "Software Engineer",
		});

		expect(employeeProfile).toMatchObject({
			employeeId: expect.any(String),
			name: "Jane Smith",
			email: "jane@growingcorp.com",
			position: "Software Engineer",
			companyId: expect.any(String),
			hiredAt: expect.any(Number),
		});

		// Verify employee is in company's list
		const employees = await company.getEmployees();
		expect(employees).toContain("jane@growingcorp.com");
	});

	test("get employee profile directly", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company and employee
		const company = await client.company.create(["45-6789012"], {
			input: {
				name: "Test Corp",
				industry: "Testing",
			},
		});

		await company.createEmployee({
			name: "John Doe",
			email: "john@testcorp.com",
			position: "QA Engineer",
		});

		// Get employee directly using email key
		const employee = client.employee.get(["john@testcorp.com"]);
		const profile = await employee.getProfile();

		expect(profile).toMatchObject({
			name: "John Doe",
			email: "john@testcorp.com",
			position: "QA Engineer",
		});
	});

	test("update employee profile", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company and employee
		const company = await client.company.create(["56-7890123"], {
			input: {
				name: "Update Corp",
				industry: "Technology",
			},
		});

		await company.createEmployee({
			name: "Alice Johnson",
			email: "alice@updatecorp.com",
			position: "Junior Developer",
		});

		// Update employee profile
		const employee = client.employee.get(["alice@updatecorp.com"]);
		const updatedProfile = await employee.updateProfile({
			name: "Alice Johnson-Smith",
			position: "Senior Developer",
		});

		expect(updatedProfile.name).toBe("Alice Johnson-Smith");
		expect(updatedProfile.position).toBe("Senior Developer");
	});

	test("company tracks multiple employees", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create company
		const company = await client.company.create(["67-8901234"], {
			input: {
				name: "Multi Corp",
				industry: "Technology",
			},
		});

		// Create multiple employees
		await company.createEmployee({
			name: "Employee One",
			email: "one@multicorp.com",
			position: "Developer",
		});

		await company.createEmployee({
			name: "Employee Two",
			email: "two@multicorp.com",
			position: "Designer",
		});

		await company.createEmployee({
			name: "Employee Three",
			email: "three@multicorp.com",
			position: "Manager",
		});

		// Verify all employees are tracked
		const employees = await company.getEmployees();
		expect(employees).toHaveLength(3);
		expect(employees).toContain("one@multicorp.com");
		expect(employees).toContain("two@multicorp.com");
		expect(employees).toContain("three@multicorp.com");
	});
});
