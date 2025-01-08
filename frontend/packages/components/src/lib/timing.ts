export const timing = {
	milliseconds(v: number) {
		return Math.floor(v);
	},
	seconds(v: number) {
		return this.milliseconds(v * 1000);
	},
	minutes(v: number) {
		return this.seconds(v * 60);
	},
	hours(v: number) {
		return this.minutes(v * 60);
	},
	days(v: number) {
		return this.hours(v * 24);
	},
};

export function millisecondsToMonths(milliseconds: number) {
	// Calculate the difference in months
	const months = milliseconds / timing.days(31);

	return months;
}
