import { type ClassValue, clsx } from "clsx";
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
	actorsList: {
		set: (width: number, folded: boolean) => {
			ls.set("actors-list-preview-width", width);
			ls.set("actors-list-preview-folded", folded);
		},
		getWidth: () => ls.get("actors-list-preview-width"),
		getFolded: () => ls.get("actors-list-preview-folded"),
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

export function endWithSlash(url: string) {
	return url.endsWith("/") ? url : `${url}/`;
}
