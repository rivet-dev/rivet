export function remToPx(remValue) {
	const rootFontSize =
		typeof window === "undefined"
			? 16
			: Number.parseFloat(
					window.getComputedStyle(document.documentElement).fontSize,
				);

	return Number.parseFloat(remValue) * rootFontSize;
}
