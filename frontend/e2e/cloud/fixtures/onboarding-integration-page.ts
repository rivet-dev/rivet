import { expect, type Page } from "@playwright/test";
import { TEST_IDS } from "../../../src/utils/test-ids";

export class OnboardingIntegrationPage {
	constructor(private page: Page) {}

	async waitForProviderStep() {
		await expect(
			this.page.getByTestId(
				TEST_IDS.Onboarding.IntegrationProviderSelection,
			),
		).toBeVisible();
	}

	async selectProvider(providerName: string) {
		const providerOption = this.page.getByTestId(
			TEST_IDS.Onboarding.IntegrationProviderOption(providerName),
		);
		await providerOption.click();
	}

	async fillEndpoint(url: string) {
		await this.page.getByLabel(/endpoint/i).fill(url);
	}

	async selectFirstDatacenter() {
		// Open the datacenter combobox and pick the first available option
		await this.page.getByRole("combobox").click();
		await this.page.getByRole("option").first().click();
	}

	async assertConnectionSuccess() {
		await expect(
			this.page.getByText(/is running with RivetKit/i),
		).toBeVisible({ timeout: 10_000 });
	}

	async assertConnectionFailure() {
		await expect(
			this.page.getByText(/Health check failed, verify/i),
		).toBeVisible({ timeout: 10_000 });
	}

	async assertConnectionPending() {
		await expect(
			this.page.getByText(/Waiting for Runner to connect/i),
		).toBeVisible({ timeout: 10_000 });
	}

	async submitBackendStep() {
		// The stepper Next button is an icon-only submit button.
		// Wait for it to be enabled (form becomes valid after successful connection check).
		const submitButton = this.page.locator(
			'[data-component="stepper"] button[type="submit"]',
		);
		await submitButton.waitFor({ state: "visible" });
		await expect(submitButton).toBeEnabled({ timeout: 10_000 });
		await submitButton.click();
	}

	async waitForVerificationStep() {
		await expect(
			this.page.getByTestId(TEST_IDS.Onboarding.VerificationStep),
		).toBeVisible();
	}
}
