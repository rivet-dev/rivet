import { expect, test as setup } from "@playwright/test";

const authFile = ".auth/cloud/user.json";

setup("authenticate", async ({ page, request }) => {
	// Get credentials from environment
	const email = process.env.E2E_USER_EMAIL;
	const password = process.env.E2E_USER_PASSWORD;

	if (!email || !password) {
		throw new Error(
			"E2E_USER_EMAIL and E2E_USER_PASSWORD must be set in .env.local",
		);
	}

	// Sign in via Better Auth API endpoint
	const baseURL = "http://localhost:43710";
	const response = await request.post(
		`${baseURL}/api/auth/sign-in/email`,
		{
			data: { email, password },
			headers: { "Content-Type": "application/json" },
		},
	);

	expect(response.ok()).toBeTruthy();

	// Navigate to trigger cookie storage in browser context
	await page.goto("/");

	// Wait for redirect away from login (session cookies are set)
	await expect(page).not.toHaveURL(/login/);

	// Save authentication state (cookies + storage)
	await page.context().storageState({ path: authFile });
});
