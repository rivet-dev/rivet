import { expect, type Page } from "@playwright/test";
import { TEST_IDS } from "../../../src/utils/test-ids";

export class OnboardingPage {
	constructor(private page: Page) {}

	async navigateToNewProject() {
		// Mock runner configs + runners to return empty so the wizard always
		// shows for a fresh project regardless of the test account's state.
		await this.page.route(/\/runner-configs(\?|$)/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ runnerConfigs: {}, cursor: null }),
			}),
		);
		await this.page.route(/\/runners\/names(\?|$)/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ names: [], cursor: null }),
			}),
		);
		await this.page.route(/\/actors\/names(\?|$)/, (route) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ names: {}, cursor: null }),
			}),
		);
		await this.page.goto("/new");
		// /new redirects to /orgs/$orgId/new — wait for create project card
		await expect(
			this.page.getByTestId(TEST_IDS.Onboarding.CreateProjectCard),
		).toBeVisible({ timeout: 10_000 });
	}

	async createProject(name: string) {
		const createProjectCard = this.page.getByTestId(
			TEST_IDS.Onboarding.CreateProjectCard,
		);
		await expect(createProjectCard).toBeVisible();

		await createProjectCard.getByLabel(/name/i).fill(name);
		await createProjectCard
			.getByRole("button", { name: /create/i })
			.click();
	}

	async waitForWizardMount() {
		await expect(
			this.page.getByTestId(TEST_IDS.Onboarding.GettingStartedWizard),
		).toBeVisible({ timeout: 15_000 });
	}

	async assertActiveStep(stepTitle: string) {
		await expect(
			this.page.getByRole("heading", { name: stepTitle }),
		).toBeVisible();
	}

	async clickNext() {
		// The stepper Next button is an icon-only submit button
		await this.page
			.locator('[data-component="stepper"] button[type="submit"]')
			.click();
	}

	async skipToDeploy() {
		await this.page
			.getByTestId(TEST_IDS.Onboarding.StepperSkipToDeploy)
			.click();
	}

	getByTestId(testId: string) {
		return this.page.getByTestId(testId);
	}
}
