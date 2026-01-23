import { expect } from "@playwright/test";
import { test } from "./fixtures";

test.describe("new user onboarding", () => {
	// we need to assume that the user has no orgs or projects yet
	// for now we re-use the user credentials from other tests
	// in the future we might want to create a fresh user via Clerk API

	// Note: These tests focus on the onboarding flow starting from /new
	// and do not cover authentication or organization creation steps
	test("when selecting the coding agent onboarding path, user can create an agentic project successfully", async ({
		onboardingPage,
		page,
	}) => {
		await onboardingPage.selectAgentPath();
		await onboardingPage.createProject("Agentic Project");
		// should be redirected to the new project page
		await expect(page).toHaveURL(/orgs\/[^/]+\/projects\/agentic-project/);
	});

	test("when selecting the template onboarding path, user can create a templated project successfully", async ({
		onboardingPage,
		page,
	}) => {
		await onboardingPage.selectTemplatePath();
		await onboardingPage.selectTemplate("chat-room");
		await onboardingPage.createTemplateProject("Templated Project");
		// should be redirected to the new project page
		await expect(page).toHaveURL(
			/orgs\/[^/]+\/projects\/templated-project/,
		);
	});

	test("when selecting the manual onboarding path, user can create a manual project successfully", async ({
		onboardingPage,
		page,
	}) => {
		await onboardingPage.selectManualPath();
		await onboardingPage.createProject("Manual Project");
		// should be redirected to the new project page
		await expect(page).toHaveURL(/orgs\/[^/]+\/projects\/manual-project/);
	});
});
