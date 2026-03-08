import { test as base, type Page } from "@playwright/test";
import { TEST_IDS } from "../../../src/utils/test-ids";

type Fixtures = {
	authenticated: Page;
};

export const test = base.extend<Fixtures>({
	authenticated: async ({ page }, use) => {
		await page.goto("/ui");
		await page.getByTestId(TEST_IDS.Engine.AdminTokenForm).waitFor();
		await page
			.getByLabel("Token")
			.fill(process.env.E2E_ADMIN_TOKEN ?? "invalid-token");
		await page
			.getByTestId(TEST_IDS.Engine.AdminTokenForm)
			.getByRole("button")
			.click();
		await page
			.getByTestId(TEST_IDS.Engine.AdminTokenForm)
			.waitFor({ state: "detached" });
		await page
			.getByText("Successfully authenticated with Rivet Engine")
			.waitFor();
		await use(page);
	},
});
