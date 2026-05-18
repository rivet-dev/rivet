import { useCallback, useEffect, useState } from "react";

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
	const stored = window.localStorage.getItem(STORAGE_KEY);
	if (stored === "light" || stored === "dark") return stored;
	return "dark";
}

export function useTheme() {
	const [theme, setTheme] = useState<Theme>(() => readStoredTheme());

	useEffect(() => {
		applyTheme(theme);
		try {
			window.localStorage.setItem(STORAGE_KEY, theme);
		} catch {
			// Storage might be unavailable (private mode, quota). Best-effort only.
		}
	}, [theme]);

	const toggle = useCallback(() => {
		setTheme((prev) => (prev === "dark" ? "light" : "dark"));
	}, []);

	return { theme, setTheme, toggle };
}
