export async function authenticate(token: string): Promise<string> {
	// Mock authentication - in real app, verify JWT or session token
	if (token === "demo-token") {
		return "user123";
	}
	throw new Error("Invalid token");
}
