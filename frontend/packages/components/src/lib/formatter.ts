export interface DurationOptions {
	showSeconds?: boolean;
	showMilliseconds?: boolean;
	shorten?: boolean;
	show0Min?: boolean;
}

export function formatDuration(duration: number, opts: DurationOptions = {}) {
	const negative = Math.sign(duration) === -1;
	const dur = Math.abs(duration);

	// Decompose duration
	const milliseconds = dur % 1000;
	const seconds = Math.floor(dur / 1000);
	const minutes = Math.floor(seconds / 60);
	const hours = Math.floor(minutes / 60);
	const days = Math.floor(hours / 24);
	const years = Math.floor(days / 365);

	// Format string
	const s = [];
	if (years > 0) s.push(`${years}y`);
	if (days > 0) s.push(`${days % 365}d`);
	if (hours > 0) s.push(`${hours % 24}h`);

	if (opts.showSeconds) {
		if (minutes > 0) s.push(`${minutes % 60}m`);
		if (seconds >= 0) s.push(`${seconds % 60}s`);
	} else {
		if (minutes > 0) s.push(`${minutes % 60}m`);
		// Make sure it says at least "1m"
		else if (!s.length) s.push(opts.show0Min ? "0m" : "1m");
	}

	if (opts.showMilliseconds && milliseconds) s.push(`${dur % 1000}ms`);

	return `${negative ? "-" : ""}${(opts.shorten ? s.slice(0, 2) : s).join(" ")}`;
}

const currencyFormatter = new Intl.NumberFormat("en-US", {
	style: "currency",
	currency: "USD",
});

export function formatCurrency(amount: number) {
	return currencyFormatter.format(amount);
}

export function formatCurrencyToParts(amount: number) {
	return currencyFormatter.formatToParts(amount);
}
