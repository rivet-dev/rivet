import { createClient } from "rivetkit/client";
import type { registry } from "./quickstart/registry";

const client = createClient<typeof registry>();
const userId = "user_alice";
const profile = client.userProfile.getOrCreate(["user", userId]);

// Reach exactly one actor. This is the default, so `permissions` can be
// omitted entirely. The holder cannot create actors or discover others.
export const oneActor = () => profile.issueToken({ subject: userId });

// Widen what the holder may do to that same actor. Every grant stays scoped
// to its resolved ID.
export const oneActorWithKv = () =>
	profile.issueToken({
		subject: userId,
		permissions: {
			actor_gateway: ["read"],
			actor_kv: ["read"],
		},
	});

// Namespace-wide operations such as creating actors need an explicit grant
// list. Nothing is added automatically.
export const anyActorInNamespace = () =>
	client.auth.issueToken({
		subject: userId,
		grants: [
			{
				resource: "actor",
				target: "any",
				operations: ["create", "read"],
			},
			{ resource: "actor_gateway", target: "any", operations: ["read"] },
		],
	});
