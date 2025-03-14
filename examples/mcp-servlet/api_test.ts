import { type Rivet, RivetClient } from "@rivet-gg/api";

const token = process.env.RIVET_SERVICE_TOKEN;
if (!token) throw "missing RIVET_SERVICE_TOKEN";
const project = process.env.RIVET_PROJECT;
if (!project) throw "missing RIVET_PROJECT";
const environment = process.env.RIVET_ENVIRONMENT;
if (!environment) throw "missing RIVET_ENVIRONMENT";

const client = new RivetClient({ token });

let actor: Rivet.actor.Actor | undefined = undefined;

try {
	console.log("Creating actor");
	actor = (
		await client.actor.create({
			project,
			environment,
			body: {
				tags: {
					foo: "bar",
				},
				buildTags: { name: "simple_http", current: "true" },
				network: {
					ports: {
						http: {
							protocol: "https",
						},
					},
				},
			},
		})
	).actor;

	const port = actor.network.ports.http;
	if (!port) throw "Missing http port";
	console.log("Connecting to actor", port.url);
	const res = await fetch(port.url);
	if (!res.ok) throw `Failed to request actor: ${res.statusText}`;
	const resText = await res.text();
	console.log("Actor response", resText);
} finally {
	if (actor) {
		//console.log("Destroying actor");
		//await client.actor.destroy(actor.id);
	}
}
