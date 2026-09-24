#!/usr/bin/env node
import { parseArgs } from "node:util";
import type { Credential } from "@earendil-works/pi-ai";
import { createClient } from "rivetkit/client";
import type { PiAgentHandle } from "./agent.js";
import { loginInTerminal } from "./login.js";
import { serveAcp } from "./serve.js";

const USAGE = `Usage:
  rivet-pi acp --actor <name> [--user <id>] [--credentials <name>]
  rivet-pi acp --actor <name> [--user <id>] --credentials <name> login

Connects with RivetKit's standard settings: RIVET_ENDPOINT, RIVET_TOKEN, RIVET_NAMESPACE.
Each ACP session is the Pi actor with key [user, sessionId].
With --credentials, editors offer a terminal login. It calls save(provider, credential)
on the actor <name> with key [user].`;

type ActorAccessor<THandle> = {
	getOrCreate(key: string[]): THandle;
	get(key: string[]): THandle;
};

type CredentialsHandle = {
	save(provider: string, credential: Credential): Promise<unknown>;
};

const { positionals, values } = parseArgs({
	allowPositionals: true,
	options: {
		actor: { type: "string" },
		user: { type: "string", default: "default" },
		credentials: { type: "string" },
		help: { type: "boolean" },
	},
});
const [command, subcommand] = positionals;
const user = values.user!;
const credentialsActor = values.credentials;

if (values.help || command !== "acp" || !values.actor || (subcommand === "login" && !credentialsActor)) {
	process.stderr.write(`${USAGE}\n`);
	process.exit(values.help ? 0 : 1);
}

if (subcommand === "login") {
	const { provider, credential } = await loginInTerminal();
	const credentials = (createClient() as unknown as Record<string, ActorAccessor<CredentialsHandle>>)[credentialsActor!]!;
	await credentials.getOrCreate([user]).save(provider, credential);
	process.stderr.write(`Saved the ${provider} login.\n`);
	process.exit(0);
}

const actorName = values.actor;
let agents: ActorAccessor<PiAgentHandle> | undefined;
serveAcp({
	actor: (sessionId, create) => {
		agents ??= (createClient() as unknown as Record<string, ActorAccessor<PiAgentHandle>>)[actorName]!;
		return create ? agents.getOrCreate([user, sessionId]) : agents.get([user, sessionId]);
	},
	...(credentialsActor && {
		login: { command: process.execPath, commandArgs: process.argv.slice(1), loginArgs: ["login"] },
	}),
});
