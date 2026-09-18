const CAL_VARS: Record<string, string> = {
	"cal-brand": "--primary",
	"cal-bg": "--card",
	"cal-bg-emphasis": "--muted",
	"cal-bg-subtle": "--card",
	"cal-bg-muted": "--card",
	"cal-border": "--border",
	"cal-border-subtle": "--border",
	"cal-border-emphasis": "--border",
	"cal-text": "--foreground",
	"cal-text-emphasis": "--foreground",
	"cal-text-subtle": "--muted-foreground",
	"cal-text-muted": "--muted-foreground",
};

function hslTokenToHex(token: string) {
	const [h, s, l] = token
		.replaceAll("deg", "")
		.replaceAll("%", "")
		.split(/\s+/)
		.map(Number);
	if ([h, s, l].some((part) => !Number.isFinite(part))) {
		return undefined;
	}
	const saturation = s / 100;
	const lightness = l / 100;
	const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
	const channel = (n: number) => {
		const k = (n + h / 30) % 12;
		const value =
			lightness - (chroma / 2) * Math.max(-1, Math.min(k - 3, 9 - k, 1));
		return Math.round(value * 255)
			.toString(16)
			.padStart(2, "0");
	};
	return `#${channel(0)}${channel(8)}${channel(4)}`;
}

export function readCalVars() {
	if (typeof document === "undefined") return {};
	const styles = getComputedStyle(document.documentElement);
	const vars: Record<string, string> = {};
	for (const [calVar, token] of Object.entries(CAL_VARS)) {
		const hex = hslTokenToHex(styles.getPropertyValue(token).trim());
		if (hex) {
			vars[calVar] = hex;
		}
	}
	return vars;
}
