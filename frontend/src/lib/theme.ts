import { useCallback, useSyncExternalStore } from "react";

export type Theme = "light" | "dark";

const STORAGE_KEY = "rivet:theme";

function applyTheme(theme: Theme) {
	const root = document.documentElement;
	if (theme === "dark") {
		root.classList.add("dark");
	} else {
		root.classList.remove("dark");
	}
}

function readStoredTheme(): Theme {
	if (typeof window === "undefined") return "dark";
	const fromUrl = new URLSearchParams(window.location.search).get("theme");
	if (fromUrl === "light" || fromUrl === "dark") return fromUrl;
	const stored = window.localStorage.getItem(STORAGE_KEY);
	if (stored === "light" || stored === "dark") return stored;
	return "dark";
}

// One-time bootstrap: ensure the DOM matches the persisted preference on
// first import. Done lazily so SSR doesn't crash on a missing `document`.
let bootstrapped = false;
function bootstrap() {
	if (bootstrapped) return;
	if (typeof document === "undefined") return;
	bootstrapped = true;
	applyTheme(readStoredTheme());
}

// `<html>` carries the active theme as a class. Every `useTheme()` caller
// reads from the live DOM via `useSyncExternalStore`, so when one caller
// flips the class every other subscriber re-renders immediately — no
// stale per-`useState` copies.
function getSnapshot(): Theme {
	if (typeof document === "undefined") return "dark";
	return document.documentElement.classList.contains("dark")
		? "dark"
		: "light";
}

function getServerSnapshot(): Theme {
	return "dark";
}

function subscribe(callback: () => void): () => void {
	if (typeof document === "undefined") return () => {};
	const observer = new MutationObserver(callback);
	observer.observe(document.documentElement, {
		attributes: true,
		attributeFilter: ["class"],
	});
	return () => observer.disconnect();
}

export function useTheme() {
	bootstrap();
	const theme = useSyncExternalStore(
		subscribe,
		getSnapshot,
		getServerSnapshot,
	);

	const setTheme = useCallback((next: Theme) => {
		applyTheme(next);
		try {
			window.localStorage.setItem(STORAGE_KEY, next);
		} catch {
			// Storage might be unavailable (private mode, quota).
			// Best-effort only.
		}
	}, []);

	const toggle = useCallback(() => {
		setTheme(getSnapshot() === "dark" ? "light" : "dark");
	}, [setTheme]);

	return { theme, setTheme, toggle };
}
