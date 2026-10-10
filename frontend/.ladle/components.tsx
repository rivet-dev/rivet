import type { GlobalProvider } from "@ladle/react";

/**
 * Sync Ladle's theme addon with the dashboard's `.dark` token scope.
 *
 * Ladle sets `data-theme` on `<html>`; the app's `theme.css` keys dark tokens
 * off `:root.dark`, so mirror the state onto the class. Writing to the DOM
 * during render is fine here: it is idempotent and must happen before the
 * story paints to avoid a light/dark flash.
 */
export const Provider: GlobalProvider = ({ children, globalState }) => {
	document.documentElement.classList.toggle(
		"dark",
		globalState.theme === "dark",
	);
	return children;
};
