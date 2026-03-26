import { setupClerkTestingToken } from "@clerk/testing/playwright";
import { test as base, type Page } from "@playwright/test";
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
		const page = new OnboardingPage(authenticated);
		await page.navigateToNewProject();
		await use(page);
	},
	onboardingIntegrationPage: async ({ authenticated }, use) => {
		await use(new OnboardingIntegrationPage(authenticated));
	},
});
