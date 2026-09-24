// Grant sets for three common scopes. Pass one as the `grants` field when
// issuing a token.

type Grant = {
	resource: "actor" | "actor_gateway" | "actor_kv";
	target: "any" | { id: string };
	operations: Array<"create" | "read" | "update" | "delete" | "list">;
};

// Reach exactly one actor. The holder cannot create actors or discover others.
export function oneActor(actorId: string): Grant[] {
	return [
		{
			resource: "actor_gateway",
			target: { id: actorId },
			operations: ["read"],
		},
	];
}

// Open or join any actor the client names. Per-user rules must then live in
// the actor itself.
export function anyActorInNamespace(): Grant[] {
	return [
		{ resource: "actor", target: "any", operations: ["create", "read"] },
		{ resource: "actor_gateway", target: "any", operations: ["read"] },
	];
}

// Read one actor and its raw KV, for an inspector or admin view.
export function oneActorWithKv(actorId: string): Grant[] {
	return [
		{
			resource: "actor_gateway",
			target: { id: actorId },
			operations: ["read"],
		},
		{ resource: "actor_kv", target: { id: actorId }, operations: ["read"] },
	];
}
