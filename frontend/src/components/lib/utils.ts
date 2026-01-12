import { type ClassValue, clsx } from "clsx";
import { set } from "lodash";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export const ls = {
	set: (key: string, value: unknown) => {
		localStorage.setItem(key, JSON.stringify(value));
	},
	get: (key: string) => {
		const value = localStorage.getItem(key);
		try {
			return value ? JSON.parse(value) : null;
		} catch {
			return null;
		}
	},
	remove: (key: string) => {
		localStorage.removeItem(key);
	},
	clear: () => {
		localStorage.clear();
	},
	engineCredentials: {
		key: (url: string) => btoa(`engine-credentials-${JSON.stringify(url)}`),
		set: (url: string, token: string) => {
			ls.set(
				btoa(`engine-credentials-${JSON.stringify(url)}`),
				JSON.stringify({ token }),
			);
		},
		get: (url: string) => {
			try {
				const value = JSON.parse(
					ls.get(btoa(`engine-credentials-${JSON.stringify(url)}`)),
				);
				if (value && typeof value === "object" && "token" in value) {
					return (value as { token: string }).token;
				}
			} catch {
				return null;
			}
		},
	},
	actorsList: {
		set: (width: number, folded: boolean) => {
			ls.set("actors-list-preview-width", width);
			ls.set("actors-list-preview-folded", folded);
		},
		getWidth: () => ls.get("actors-list-preview-width"),
		getFolded: () => ls.get("actors-list-preview-folded"),
	},
	actorsEphemeralFilters: {
		key: "actors-ephemeral-filters",
		set: (filters: Record<string, unknown>) => {
			ls.set(ls.actorsEphemeralFilters.key, JSON.stringify(filters));
		},
		get: () => {
			try {
				return JSON.parse(
					ls.get(ls.actorsEphemeralFilters.key),
				) as Record<string, unknown> | null;
			} catch {
				return {};
			}
		},
	},
};

export function toRecord(value: unknown) {
	if (typeof value === "object" && value !== null) {
		return value as Record<string, unknown>;
	}

	return {};
}

export function assertNonNullable<V>(v: V): asserts v is Exclude<V, null> {
	if (!v) {
		throw new Error(`${v} is null`);
	}
}

export function assertUnreachable(_x: never): never {
	throw new Error("Didn't expect to get here");
}

export function endWithSlash(url: string) {
	return url.endsWith("/") ? url : `${url}/`;
}
