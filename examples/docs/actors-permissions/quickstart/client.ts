import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>();

const doc = client.document.getOrCreate(["welcome"], {
	params: { authToken: "member-token" },
});

const conn = doc.connect();

// Allowed: every authenticated caller may read.
console.log(await conn.read());

try {
	// Rejected: this connection is a member, not an admin.
	await conn.edit("hello");
} catch (error) {
	console.error(error);
}
