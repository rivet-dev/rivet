import { expect } from "@playwright/test";
import { TEST_IDS } from "../../src/utils/test-ids";
import { test } from "./fixtures";

test.describe("project creation", () => {
	test("user can create a project and reach the onboarding wizard", async ({
		onboardingPage,
		page,
	}) => {
		await onboardingPage.createProject("Test Project");
		// should be redirected to the namespace route
		await expect(page).toHaveURL(/orgs\/[^/]+\/projects\/test-project/);
		// wizard should mount at the namespace route
		await onboardingPage.waitForWizardMount();
	});
});

test.describe("onboarding wizard", () => {
	test("provider selection grid renders all expected providers", async ({
		onboardingPage,
	}) => {
		await onboardingPage.createProject("Provider Test Project");
		await onboardingPage.waitForWizardMount();

		// skip local steps to reach provider selection
		await onboardingPage.skipToDeploy();

		// all expected providers should be visible (cloudflare-workers is filtered out as specializedPlatform)
		for (const provider of [
			"vercel",
			"gcp-cloud-run",
			"railway",
			"hetzner",
			"aws-ecs",
			"kubernetes",
		]) {
			await expect(
				onboardingPage.getByTestId(
					TEST_IDS.Onboarding.IntegrationProviderOption(provider),
				),
			).toBeVisible();
		}
	});

	test("skip to deploy link jumps from local step to provider step", async ({
		onboardingPage,
		page,
	}) => {
		await onboardingPage.createProject("Skip Deploy Test Project");
		await onboardingPage.waitForWizardMount();

		// skip to deploy should be visible on install step (first local step)
		await expect(
			page.getByTestId(TEST_IDS.Onboarding.StepperSkipToDeploy),
		).toBeVisible();

		await onboardingPage.skipToDeploy();

		// provider selection grid should now be visible
		await expect(
			page.getByTestId(TEST_IDS.Onboarding.IntegrationProviderSelection),
		).toBeVisible();
	});

	test("backend step Next button becomes enabled after successful connection check and datacenter selection", async ({
		onboardingPage,
		onboardingIntegrationPage,
		page,
	}) => {
		// Mock datacenters to return a known region so the datacenter combobox is populated.
		// Must include pagination and use the correct Datacenter shape (label, name, url).
		await page.route(/\/datacenters(\?|$)/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({
					datacenters: [
						{ label: 1, name: "atl", url: "https://atl.rivet.run" },
					],
					pagination: { cursor: null },
				}),
			}),
		);

		// Mock health check to return success
		await page.route(/runner-configs\/serverless-health-check/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ success: { version: "1.0.0" } }),
			}),
		);

		await onboardingPage.createProject(
			"Next Button Regression Test Project",
		);
		await onboardingPage.waitForWizardMount();

		await onboardingPage.skipToDeploy();
		await onboardingIntegrationPage.waitForProviderStep();
		await onboardingIntegrationPage.selectProvider("vercel");
		await onboardingPage.clickNext();

		// Select a datacenter (required by configurationSchema)
		await onboardingIntegrationPage.selectFirstDatacenter();

		// Fill a valid endpoint
		await onboardingIntegrationPage.fillEndpoint(
			"https://my-app.vercel.app",
		);

		// Wait for connection success — sets success=true in the form
		await onboardingIntegrationPage.assertConnectionSuccess();

		// The Next button must become enabled.
		// Before the fix (missing runnerName default + datacenters: []) it stayed disabled permanently.
		const submitButton = page.locator(
			'[data-component="stepper"] button[type="submit"]',
		);
		await expect(submitButton).toBeEnabled({ timeout: 10_000 });
	});

	test("selecting a provider and entering a valid endpoint shows connection success", async ({
		onboardingPage,
		onboardingIntegrationPage,
		page,
	}) => {
		// mock the runner health check endpoint to return success
		await page.route(/runner-configs\/serverless-health-check/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ success: { version: "1.0.0" } }),
			}),
		);

		await onboardingPage.createProject("Health Check Test Project");
		await onboardingPage.waitForWizardMount();

		await onboardingPage.skipToDeploy();

		await onboardingIntegrationPage.waitForProviderStep();
		await onboardingIntegrationPage.selectProvider("vercel");

		// click next to go to backend config step
		await onboardingPage.clickNext();

		// fill in a valid endpoint
		await onboardingIntegrationPage.fillEndpoint(
			"https://my-app.vercel.app",
		);

		// should show connection check success
		await onboardingIntegrationPage.assertConnectionSuccess();
	});

	test("selecting a provider and entering an invalid endpoint shows failure state", async ({
		onboardingPage,
		onboardingIntegrationPage,
		page,
	}) => {
		// mock the runner health check endpoint to return failure
		await page.route(/runner-configs\/serverless-health-check/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({
					failure: {
						error: { requestFailed: {} },
					},
				}),
			}),
		);

		await onboardingPage.createProject("Failure Check Test Project");
		await onboardingPage.waitForWizardMount();

		await onboardingPage.skipToDeploy();

		await onboardingIntegrationPage.waitForProviderStep();
		await onboardingIntegrationPage.selectProvider("vercel");

		await onboardingPage.clickNext();

		await onboardingIntegrationPage.fillEndpoint(
			"https://unreachable.example.com",
		);

		await onboardingIntegrationPage.assertConnectionFailure();
	});

	test("verification step renders waiting state when backend is configured but no actors exist", async ({
		onboardingPage,
		page,
	}) => {
		// Mock runner configs to return a configured backend (so displayFrontendOnboarding=true)
		await page.route(/\/runner-configs(\?|$)/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({
					runnerConfigs: {
						default: {
							datacenters: {
								atl: {
									serverless: {
										url: "https://my-app.vercel.app/api/rivet",
										headers: {},
									},
									metadata: { provider: "vercel" },
								},
							},
						},
					},
					cursor: null,
				}),
			}),
		);

		// Mock actors list names to return empty so verification step stays in waiting state
		await page.route(/\/actors\/names/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ names: {}, cursor: null }),
			}),
		);

		// Mock runners/names to return empty (triggers hasBackendConfigured via runnerConfigs only)
		await page.route(/\/runners\/names/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ names: [], cursor: null }),
			}),
		);

		await onboardingPage.createProject("Verification Test Project");

		// With mocked runner configs, wizard should load in displayFrontendOnboarding mode
		await onboardingPage.waitForWizardMount();

		// The verification step (FrontendSetup) should be visible since backend is configured
		await expect(
			page.getByTestId(TEST_IDS.Onboarding.VerificationStep),
		).toBeVisible({ timeout: 15_000 });
		await expect(
			page.getByTestId(TEST_IDS.Onboarding.WaitingForActor),
		).toBeVisible();
	});
});
