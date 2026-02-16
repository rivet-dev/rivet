import { UserError } from "rivetkit";

export function sleep(ms: number) {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

export function err(message: string, code: string): never {
	throw new UserError(message, { code });
}
