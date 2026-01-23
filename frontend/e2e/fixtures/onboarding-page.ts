import { expect } from "@playwright/test";
import { TEST_IDS } from "../../src/utils/test-ids";

export class OnboardingPage {
	constructor(private page: import("@playwright/test").Page) {}
	async selectAgentPath() {
		await this.page
			.getByTestId(TEST_IDS.Onboarding.PathSelectionAgent)
			.click();
	}
	async selectTemplatePath() {
		await this.page
			.getByTestId(TEST_IDS.Onboarding.PathSelectionTemplate)
			.click();
	}
	async selectManualPath() {
		await this.page
			.getByTestId(TEST_IDS.Onboarding.PathSelectionManual)
			.click();
	}

	async selectTemplate(templateName: string) {
		const templateOption = this.page.getByTestId(
			TEST_IDS.Onboarding.TemplateOption(templateName),
		);
		await expect(templateOption).toBeVisible();
		await templateOption.click();
	}

	async createProject(name: string) {
		// should show create project card
		const createProjectCard = this.page.getByTestId(
			TEST_IDS.Onboarding.CreateProjectCard,
		);
		await expect(createProjectCard).toBeVisible();
		await expect(createProjectCard).toHaveScreenshot(
			"onboarding-create-project-card.png",
		);

		// should be able to create project
		await createProjectCard.getByLabel(/name/i).fill(name);
		await createProjectCard
			.getByRole("button", { name: /create/i })
			.click();
	}

	async createTemplateProject(name: string) {
		// should show create project card
		const createProjectCard = this.page.getByTestId(
			TEST_IDS.Onboarding.CreateTemplateProjectCard,
		);
		await expect(createProjectCard).toBeVisible();
		await expect(createProjectCard).toHaveScreenshot(
			"onboarding-create-template-project-card.png",
		);

		// should be able to create project
		await createProjectCard.getByLabel(/name/i).fill(name);
		await createProjectCard
			.getByRole("button", { name: /create/i })
			.click();
	}

	getByTestId(testId: string) {
		return this.page.getByTestId(testId);
	}
}
