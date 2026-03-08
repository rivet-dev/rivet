import { expect } from "@playwright/test";
import { TEST_IDS } from "../../src/utils/test-ids";
import { test } from "./fixtures";

test.use({ baseURL: `http://localhost:43708/ui` });

test.describe("initial load", () => {
	test("when not authenticated, should prompt user for admin token and authenticate successfully", async ({
		authenticated,
	}) => {
		await authenticated;
	});
	test("when authenticated, should redirect to the first available namespace", async ({
		authenticated,
	}) => {
		await authenticated.reload();
		// should be redirected to the first namespace page
		await authenticated.waitForURL("/ui/ns/*");

		// should have picked the first available actor name
		await authenticated.waitForURL((url) => url.searchParams.has("n"));

		await authenticated
			.getByTestId(TEST_IDS.Layout.Sidebar)
			.waitFor({ timeout: 10_000 });
		await authenticated.getByTestId(TEST_IDS.Layout.Main).waitFor();
	});
});
