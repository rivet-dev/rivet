import { createAuthClient } from "better-auth/client";

export const authClient = createAuthClient();

export async function login(credentials: { email: string; password: string }) {
	const { error } = await authClient.signIn.email(credentials);
	if (error) throw new Error(error.message ?? "Login failed");
	return getUserActor();
}

export async function signUp(credentials: { email: string; password: string }) {
	const { error } = await authClient.signUp.email({
		...credentials,
		name: credentials.email,
	});
	if (error) throw new Error(error.message ?? "Signup failed");
	return getUserActor();
}

async function getUserActor() {
	const response = await fetch("/api/user");
	if (!response.ok) throw new Error("Could not load your user actor");
	return (await response.json()) as {
		endpoint: string;
		namespace: string;
		actorId: string;
	};
}

export async function getActorToken() {
	// The browser sends Better Auth's session cookie automatically.
	const response = await fetch("/api/token", { method: "POST" });
	if (!response.ok) {
		throw new Error(
			response.status === 401
				? "Your login session expired. Log in again."
				: "Could not renew actor access. Please try again.",
			{ cause: response.status },
		);
	}
	return ((await response.json()) as { token: string }).token;
}
