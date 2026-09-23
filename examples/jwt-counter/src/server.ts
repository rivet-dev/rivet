import { createHash, timingSafeEqual } from "node:crypto";
import {
	createServer,
	type IncomingMessage,
	type ServerResponse,
} from "node:http";
import { createClient } from "rivetkit/client";
import { registry } from "./counter.ts";

const endpoint = required("RIVET_ENDPOINT");
const namespace = required("RIVET_NAMESPACE");
// Keep the Engine admin token on the backend; clients receive scoped JWTs only.
const adminToken = required("RIVET_ADMIN_TOKEN");
const username = required("DEMO_USER");
const password = required("DEMO_PASSWORD");
const adminClient = createClient<typeof registry>({
	endpoint,
	namespace,
	token: adminToken,
	disableMetadataLookup: true,
});

export async function handleRequest(
	req: IncomingMessage,
	res: ServerResponse,
): Promise<void> {
	res.setHeader("Cache-Control", "no-store");
	res.setHeader("Content-Type", "application/json");
	if (
		req.method !== "POST" ||
		(req.url !== "/login" && req.url !== "/token")
	) {
		res.writeHead(404).end(JSON.stringify({ error: "not_found" }));
		return;
	}

	// Basic auth is deliberately limited to this single-user CLI demonstration.
	// Use a real user/session system for a browser application; require HTTPS off loopback.
	const auth = req.headers.authorization;
	const expected = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;
	if (!auth || !equal(auth, expected)) {
		res.writeHead(401, {
			"WWW-Authenticate": 'Basic realm="JWT counter"',
		}).end(JSON.stringify({ error: "unauthorized" }));
		return;
	}

	try {
		const actorId = await adminClient.counter
			.getOrCreate(["user", username])
			.resolve();
		if (req.url === "/login") {
			res.writeHead(200).end(JSON.stringify({ actorId }));
			return;
		}

		const response = await fetch(new URL("/auth/tokens", endpoint), {
			method: "POST",
			headers: {
				Authorization: `Bearer ${adminToken}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				namespace,
				subject: username,
				duration: 30,
				grants: [
					{
						resource: "actor_gateway",
						target: { id: actorId },
						operations: ["read"],
					},
				],
			}),
		});
		if (!response.ok)
			throw new Error(`token issuance failed: HTTP ${response.status}`);
		const issued = (await response.json()) as { token: string };
		res.writeHead(200).end(JSON.stringify({ token: issued.token }));
	} catch {
		// Never expose the admin token, JWT, or backend response to the client.
		res.writeHead(503).end(JSON.stringify({ error: "issuer_unavailable" }));
	}
}

function equal(a: string, b: string): boolean {
	return timingSafeEqual(
		createHash("sha256").update(a).digest(),
		createHash("sha256").update(b).digest(),
	);
}

function required(name: string): string {
	const value = process.env[name];
	if (!value) throw new Error(`${name} must be configured`);
	return value;
}

if (import.meta.url === `file://${process.argv[1]}`) {
	if (
		!endpoint.startsWith("https://") &&
		!endpoint.startsWith("http://127.0.0.1:") &&
		!endpoint.startsWith("http://localhost:")
	) {
		throw new Error(
			"RIVET_ENDPOINT must use HTTPS except for a local Engine",
		);
	}
	registry.start();
	const host =
		process.argv.indexOf("--host") >= 0
			? process.argv[process.argv.indexOf("--host") + 1]
			: "127.0.0.1";
	createServer((req, res) => {
		void handleRequest(req, res);
	}).listen(3020, host);
}
