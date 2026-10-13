import { describe, expectTypeOf, it } from "@effect/vitest";
import { Client } from "@rivetkit/effect";
import type { GetToken } from "rivetkit/client";

describe("Client.Options", () => {
	it("accepts a getToken callback alongside endpoint/token/namespace", () => {
		const getToken: GetToken = () => Promise.resolve("scoped-token");

		const options: Client.Options = {
			endpoint: "http://127.0.0.1:6420",
			getToken,
			namespace: "default",
		};

		expectTypeOf(options.getToken).toEqualTypeOf<GetToken | undefined>();
	});
});
