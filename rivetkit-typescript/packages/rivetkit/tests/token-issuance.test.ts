import { afterEach, expect, test, vi } from "vitest";
import { createClient } from "@/client/mod";

afterEach(() => vi.unstubAllGlobals());

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "content-type": "application/json" },
	});
}

function issuedResponse(token = "scoped-token") {
	return jsonResponse({ token, issued_ts: 1_700_000_000_000, expires_ts: 1_700_000_030_000 });
}

test("client auth uses endpoint credentials and explicit grants without a duration default", async () => {
	const requests: Request[] = [];
	vi.stubGlobal("fetch", vi.fn(async (request: Request) => {
		requests.push(request);
		return issuedResponse();
	}));
	const client = createClient<any>({
		endpoint: "https://my-namespace:issuer-secret@api.example.com",
		disableMetadataLookup: true,
	});
	const grants = [{ resource: "actor", target: "any", operations: ["create"] }] as const;
	const result = await client.auth.issueToken({ grants: [...grants] });

	expect(result).toEqual({
		token: "scoped-token",
		issuedAt: 1_700_000_000_000,
		expiresAt: 1_700_000_030_000,
	});
	expect(requests).toHaveLength(1);
	expect(new URL(requests[0]!.url).pathname).toBe("/auth/tokens");
	expect(requests[0]!.headers.get("authorization")).toBe("Bearer issuer-secret");
	expect(await requests[0]!.json()).toEqual({ namespace: "my-namespace", grants });
});

test("actor handles issue ID-scoped gateway grants by default", async () => {
	const bodies: unknown[] = [];
	vi.stubGlobal("fetch", vi.fn(async (request: Request) => {
		bodies.push(await request.json());
		return issuedResponse();
	}));
	const client = createClient<any>({
		endpoint: "https://api.example.com",
		namespace: "team",
		token: "issuer",
		disableMetadataLookup: true,
	});
	const result = await client.user.getForId("actor-123").issueToken({
		subject: "user-1",
		expiresIn: 30,
	});

	expect(result.token).toBe("scoped-token");
	expect(bodies).toEqual([{
		namespace: "team",
		subject: "user-1",
		duration: 30,
		grants: [{ resource: "actor_gateway", target: { id: "actor-123" }, operations: ["read"] }],
	}]);
});

test("actor permissions replace the default and stay scoped to the resolved ID", async () => {
	const tokenBodies: unknown[] = [];
	vi.stubGlobal("fetch", vi.fn(async (request: Request) => {
		const url = new URL(request.url);
		if (url.pathname === "/actors" && request.method === "GET") {
			return jsonResponse({ actors: [{ actor_id: "resolved-id", name: "user", key: "key" }] });
		}
		if (url.pathname === "/actors" && request.method === "PUT") {
			return jsonResponse({ actor: { actor_id: "created-id", name: "user", key: "key" }, created: true });
		}
		tokenBodies.push(await request.json());
		return issuedResponse();
	}));
	const client = createClient<any>({
		endpoint: "https://api.example.com",
		namespace: "team",
		token: "issuer",
		disableMetadataLookup: true,
	});
	await client.user.get("key").issueToken({
		permissions: { actor: ["read", "update", "delete"], actor_kv: ["read"] },
	});
	await client.user.getOrCreate("other").issueToken({
		permissions: { actor_gateway: ["read"] },
	});

	expect(tokenBodies).toEqual([
		{
			namespace: "team",
			grants: [
				{ resource: "actor", target: { id: "resolved-id" }, operations: ["read", "update", "delete"] },
				{ resource: "actor_kv", target: { id: "resolved-id" }, operations: ["read"] },
			],
		},
		{
			namespace: "team",
			grants: [{ resource: "actor_gateway", target: { id: "created-id" }, operations: ["read"] }],
		},
	]);
});

test("issuance waits for metadata endpoint and credential resolution", async () => {
	const requests: Request[] = [];
	vi.stubGlobal("fetch", vi.fn(async (request: Request) => {
		requests.push(request);
		if (new URL(request.url).pathname === "/metadata") {
			return jsonResponse({
				clientEndpoint: "https://routed.example.com",
				clientNamespace: "routed-namespace",
				clientToken: "routed-token",
			});
		}
		return issuedResponse();
	}));
	const client = createClient<any>({ endpoint: "https://metadata-issuance.example.com" });
	await client.auth.issueToken({ grants: [{ resource: "actor", target: "any", operations: ["create"] }] });

	expect(requests.map((request) => new URL(request.url).host)).toEqual([
		"metadata-issuance.example.com",
		"routed.example.com",
	]);
	expect(requests[1]!.headers.get("authorization")).toBe("Bearer routed-token");
	expect((await requests[1]!.json()).namespace).toBe("routed-namespace");
});

test("issuance preserves structured Engine errors", async () => {
	vi.stubGlobal("fetch", vi.fn(async () => jsonResponse({
		group: "auth",
		code: "insufficient_permissions",
		message: "Cannot delegate grant",
	}, 403)));
	const client = createClient<any>({
		endpoint: "https://api.example.com",
		disableMetadataLookup: true,
	});
	await expect(client.auth.issueToken({
		grants: [{ resource: "actor", target: "any", operations: ["create"] }],
	})).rejects.toMatchObject({
		group: "auth",
		code: "insufficient_permissions",
		statusCode: 403,
	});
});
