import { createClient } from "rivetkit/client";
import type { registry } from "./counter.ts";

export async function runCounter(
	issuerUrl: string,
	endpoint: string,
	namespace: string,
	username: string,
	password: string,
	getTokenOverride?: (forceRefresh: boolean) => Promise<string>,
): Promise<{
	actorId: string;
	initial: number;
	incremented: number;
	current: number;
}> {
	const authorization = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;
	const issue = async (path: string) => {
		const res = await fetch(new URL(path, issuerUrl), {
			method: "POST",
			headers: { Authorization: authorization },
			cache: "no-store",
		});
		if (!res.ok)
			throw new Error(`demo authentication failed: HTTP ${res.status}`);
		return res.json();
	};
	const { actorId } = (await issue("/login")) as { actorId: string };
	const client = createClient<typeof registry>({
		endpoint,
		namespace,
		disableMetadataLookup: true,
		getToken: async ({ forceRefresh }) =>
			getTokenOverride
				? getTokenOverride(forceRefresh)
				: ((await issue("/token")) as { token: string }).token,
	});
	const counter = client.counter.getForId(actorId);
	const initial = await counter.getCount();
	const incremented = await counter.increment(1);
	return { actorId, initial, incremented, current: await counter.getCount() };
}

if (import.meta.url === `file://${process.argv[1]}`) {
	const [issuerUrl, endpoint, namespace, username, password] = [
		process.env.DEMO_ISSUER_URL ?? "http://127.0.0.1:3020",
		process.env.RIVET_ENDPOINT,
		process.env.RIVET_NAMESPACE,
		process.env.DEMO_USER,
		process.env.DEMO_PASSWORD,
	];
	if (!endpoint || !namespace || !username || !password)
		throw new Error(
			"Set RIVET_ENDPOINT, RIVET_NAMESPACE, DEMO_USER, and DEMO_PASSWORD",
		);
	const result = await runCounter(
		issuerUrl,
		endpoint,
		namespace,
		username,
		password,
	);
	console.log(
		`Counter: ${result.initial} → ${result.incremented} → ${result.current}`,
	);
}
