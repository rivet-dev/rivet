import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";
import { registry } from "../src/backend/registry";

// Mock authentication function
vi.mock("../src/backend/registry", async (importOriginal) => {
	const mod =
		await importOriginal<typeof import("../src/backend/registry")>();
	return {
		...mod,
		// We'll need to test without connection state since it requires auth
	};
});

test("Tenant organization can provide basic info", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-org"]);

	// Get organization info
	const orgInfo = await tenant.getOrganization();
	expect(orgInfo).toMatchObject({
		id: expect.any(String),
		name: expect.any(String),
		memberCount: expect.any(Number),
	});
	expect(orgInfo.memberCount).toBeGreaterThan(0);
});

test("Tenant organization tracks members", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-members"]);

	// Get all members
	const members = await tenant.getMembers();
	expect(Array.isArray(members)).toBe(true);
	expect(members.length).toBeGreaterThan(0);

	// Verify member structure
	members.forEach((member) => {
		expect(member).toMatchObject({
			id: expect.any(String),
			name: expect.any(String),
			email: expect.any(String),
			role: expect.stringMatching(/^(admin|member)$/),
		});
	});
});

test("Tenant organization provides dashboard stats", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-stats"]);

	// Get dashboard stats (without admin privileges)
	const stats = await tenant.getDashboardStats();
	expect(stats).toMatchObject({
		totalMembers: expect.any(Number),
		adminCount: expect.any(Number),
		memberCount: expect.any(Number),
	});

	// Verify member counts add up
	expect(stats.adminCount + stats.memberCount).toBe(stats.totalMembers);
	expect(stats.totalMembers).toBeGreaterThan(0);
	expect(stats.adminCount).toBeGreaterThan(0);
});

test("Tenant organization validates member roles", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-roles"]);

	const members = await tenant.getMembers();
	const orgInfo = await tenant.getOrganization();

	// Verify at least one admin exists
	const admins = members.filter((m) => m.role === "admin");
	const regularMembers = members.filter((m) => m.role === "member");

	expect(admins.length).toBeGreaterThan(0);
	expect(members.length).toBe(orgInfo.memberCount);
	expect(admins.length + regularMembers.length).toBe(members.length);
});

test("Tenant organization handles initial data correctly", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-initial-data"]);

	// Verify initial state has expected structure
	const members = await tenant.getMembers();
	const orgInfo = await tenant.getOrganization();

	expect(orgInfo.name).toBeTruthy();
	expect(orgInfo.id).toBeTruthy();
	expect(members.length).toBe(orgInfo.memberCount);

	// Verify we have the expected sample data
	expect(members.some((m) => m.role === "admin")).toBe(true);
	expect(members.some((m) => m.role === "member")).toBe(true);

	// Verify email formats
	members.forEach((member) => {
		expect(member.email).toMatch(/@/);
		expect(member.name).toBeTruthy();
		expect(member.id).toBeTruthy();
	});
});

test("Tenant organization member data consistency", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const tenant = client.tenant.getOrCreate(["test-consistency"]);

	// Get data multiple times to verify consistency
	const members1 = await tenant.getMembers();
	const members2 = await tenant.getMembers();
	const orgInfo1 = await tenant.getOrganization();
	const orgInfo2 = await tenant.getOrganization();

	expect(members1).toEqual(members2);
	expect(orgInfo1).toEqual(orgInfo2);
	expect(members1.length).toBe(orgInfo1.memberCount);
});
