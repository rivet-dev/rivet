import {
	INSPECTOR_TAB_TOKEN_NAMES,
	type InspectorTabTokens,
} from "rivetkit/inspector-tab";

/**
 * Token name of the surface custom tabs are mounted on. The iframe's
 * container is painted `bg-card`, so a tab that paints its body with this
 * token sits flush in the panel instead of one step darker/lighter.
 */
export const INSPECTOR_TAB_SURFACE = "card";

// Dashboard tokens are stored as bare HSL components (`240 7% 5%`) so they
// can be composed with alpha. Tabs get finished CSS colors instead, since
// they can't know which of the dashboard's tokens are raw.
function toCssColor(value: string): string {
	return /^(#|rgb|hsl|hwb|lab|lch|oklab|oklch|color|var\()/i.test(value)
		? value
		: `hsl(${value})`;
}

/**
 * Reads the dashboard's resolved theme colors for the theme currently
 * applied to `<html>`.
 *
 * Returns `null` if any token is missing, rather than a partial payload:
 * a tab receiving half the tokens would paint an inconsistent mix, while
 * a tab receiving none falls back to its stylesheet, which is correct.
 */
export function readInspectorTabTokens(): InspectorTabTokens | null {
	if (typeof document === "undefined") return null;
	const computed = getComputedStyle(document.documentElement);
	const tokens: Record<string, string> = {};
	for (const name of INSPECTOR_TAB_TOKEN_NAMES) {
		const raw = computed
			.getPropertyValue(
				`--${name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`,
			)
			.trim();
		if (!raw) {
			console.error(
				`inspector tab tokens: missing dashboard token --${name}, sending none`,
			);
			return null;
		}
		tokens[name] = toCssColor(raw);
	}
	return tokens as InspectorTabTokens;
}
