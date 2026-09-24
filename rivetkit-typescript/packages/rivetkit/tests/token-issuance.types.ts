import { actor } from "../src/actor/definition";
import type { ActorTokenPermissions, IssuedToken } from "../src/client/auth";
import type { Client } from "../src/client/client";
import type { Registry } from "../src/registry";

const definition = actor({
	actions: {
		issueToken: (_c, value: number) => value,
	},
});

declare const client: Client<Registry<{
	auth: typeof definition;
	user: typeof definition;
}>>;

const handle = client.user.getForId("actor-id");
const actorResult: Promise<IssuedToken> = handle.issueToken();
const clientResult: Promise<IssuedToken> = client.auth.issueToken({
	grants: [{ resource: "actor", target: "any", operations: ["create"] }],
});
void actorResult;
void clientResult;

const permissions: ActorTokenPermissions = {
	actor_gateway: ["read"],
	actor: ["read", "update", "delete"],
	actor_kv: ["read"],
};
void permissions;

// @ts-expect-error The client API requires explicit grants.
client.auth.issueToken({});
// @ts-expect-error Built-in client.auth takes precedence over an actor named auth.
client.auth.get("key");
// @ts-expect-error Built-in actor.issueToken takes precedence over an action named issueToken.
handle.issueToken(123);

const invalidResource: ActorTokenPermissions = {
	// @ts-expect-error Namespace grants belong on client.auth.issueToken.
	namespace: ["create"],
};
void invalidResource;

const invalidOperation: ActorTokenPermissions = {
	// @ts-expect-error Operation names come from Engine's generated API types.
	actor_gateway: ["connect"],
};
void invalidOperation;
