type LSKey = `rivet-lastteam-${string}` | "rivet-token";

export const ls = {
	set: (key: LSKey, value: unknown) => {
		localStorage.setItem(key, JSON.stringify(value));
	},
	get: (key: LSKey) => {
		const value = localStorage.getItem(key);
		return value ? JSON.parse(value) : null;
	},
	remove: (key: LSKey) => {
		localStorage.removeItem(key);
	},
	clear: () => {
		localStorage.clear();
	},
};
