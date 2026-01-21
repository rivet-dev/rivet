import { setupClerkTestingToken } from "@clerk/testing/playwright";
import { expect, test as setup } from "@playwright/test";

const authFile = ".auth/user.json";

setup("authenticate", async ({ page }) => {
	// Inject Clerk testing token to bypass bot detection
	await setupClerkTestingToken({ page });

	// Navigate to login page
	await page.goto("/login");

	// Get credentials from environment
	const email = process.env.E2E_CLERK_USER_EMAIL;
	const password = process.env.E2E_CLERK_USER_PASSWORD;

	if (!email || !password) {
		throw new Error(
			"E2E_CLERK_USER_EMAIL and E2E_CLERK_USER_PASSWORD must be set in .env.local",
		);
	}

	// Fill in email
	await page.getByPlaceholder("you@company.com").fill(email);
	await page.getByRole("button", { name: "Continue" }).click();

	// Wait for password step and fill in password
	await page.getByPlaceholder("Your password").fill(password);
	await page.getByRole("button", { name: "Continue" }).click();

	// Wait for successful redirect (user is logged in)
	await expect(page).not.toHaveURL(/login/);

	// Save authentication state
	await page.context().storageState({ path: authFile });
});
