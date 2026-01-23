import { setupClerkTestingToken } from "@clerk/testing/playwright";
import { test as base, expect, type Page } from "@playwright/test";
import { TEST_IDS } from "../../src/utils/test-ids";
import { OnboardingIntegrationPage } from "./onboarding-integration-page";
import { OnboardingPage } from "./onboarding-page";

type Fixtures = {
	authenticated: Page;
	onboardingPage: OnboardingPage;
	onboardingIntegrationPage: OnboardingIntegrationPage;
};

export const test = base.extend<Fixtures>({
	authenticated: async ({ page, context }, use) => {
		await setupClerkTestingToken({ page, context });
		await use(page);
	},
	onboardingPage: async ({ authenticated }, use) => {
		await authenticated.goto("/new");

		// should see path selection screen
		const pathSelection = authenticated.getByTestId(
			TEST_IDS.Onboarding.PathSelection,
		);
		await expect(pathSelection).toBeVisible();

		// should see all path options
		for (const testId of [
			TEST_IDS.Onboarding.PathSelectionAgent,
			TEST_IDS.Onboarding.PathSelectionTemplate,
			TEST_IDS.Onboarding.PathSelectionManual,
		]) {
			const path = authenticated.getByTestId(testId);
			await expect(path).toBeVisible();
		}

		await expect(pathSelection).toHaveScreenshot(
			"onboarding-path-selection.png",
		);
		await use(new OnboardingPage(authenticated));
	},
	onboardingIntegrationPage: async ({ page }, use) => {
		// should see integration instruction
		await expect(
			page.getByTestId(TEST_IDS.Onboarding.IntegrationProviderSelection),
		).toBeVisible();
		await use(new OnboardingIntegrationPage(page));
	},
});
