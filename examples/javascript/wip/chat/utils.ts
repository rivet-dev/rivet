import { UserError } from "@rivet-gg/actor";

export function validateUsername(username: string) {
	if (username.length > 16) {
		throw new UserError("Username too long");
	}
}
