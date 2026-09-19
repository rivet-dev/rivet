import { expect, it } from "vitest";
import {
	BYOC_QUICKSTART_DOCS_URL,
	BYOC_SETUP_KIT_URL,
} from "../../content/byoc";

it("links setup to the public kit and current quickstart without credentials", () => {
	expect(BYOC_SETUP_KIT_URL).toBe(
		"https://releases.rivet.dev/byoc/latest/setup-kit.tar.gz",
	);
	expect(BYOC_QUICKSTART_DOCS_URL).toBe(
		"https://rivet.dev/cloud/byoc/quickstart/",
	);
});
