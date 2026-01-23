import type { Page } from "@playwright/test";
import { TEST_IDS } from "../../src/utils/test-ids";

export class OnboardingIntegrationPage {
	constructor(private page: Page) {}

	async selectProvider(providerName: string) {
		const providerOption = this.page.getByTestId(
			TEST_IDS.Onboarding.IntegrationProviderOption(providerName),
		);
		await providerOption.click();
	}
}
