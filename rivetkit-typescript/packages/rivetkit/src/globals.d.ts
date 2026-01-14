// Minimal DOM type declarations for browser environment detection and devtools.
// We can't use TypeScript's native dom types because it pollutes the global
// type scope, overriding @types/node's types.

interface HTMLElement {
	id: string;
	appendChild(node: HTMLElement): void;
}

interface HTMLScriptElement extends HTMLElement {
	src: string;
	async: boolean;
}

declare global {
	interface Window {
		location?: {
			hostname?: string;
			origin?: string;
		};
		__rivetkit?: unknown[];
	}

	const Deno: any;
	const navigator: any;
	const window: Window | undefined;

	const document: {
		getElementById(id: string): HTMLElement | null;
		createElement(tag: "script"): HTMLScriptElement;
		head: HTMLElement;
	};
}

export {};
