import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

const counter = client.counter.getOrCreate(["my-counter"]);

// issueToken() resolves the actor ID and sends POST /auth/tokens with a
// grant scoped to this actor. It defaults to actor_gateway read access.
// expiresIn is in seconds.
const { token, expiresAt } = await counter.issueToken({
	subject: "user-123",
	expiresIn: 60 * 60,
});

// Hand the token to a browser client. It can only reach this actor.
console.log(token, new Date(expiresAt));
