export const time = {
	second(value: number): number {
		return value * 1000;
	},
	minute(value: number): number {
		return this.second(value) * 60;
	},
	hour(value: number): number {
		return this.minute(value) * 60;
	},
	day(value: number): number {
		return this.hour(value) * 24;
	},
};

export const bigTime = {
	second(value: bigint): bigint {
		return value * 1000n;
	},
	minute(value: bigint): bigint {
		return this.second(value) * 60n;
	},
	hour(value: bigint): bigint {
		return this.minute(value) * 60n;
	},
	day(value: bigint): bigint {
		return this.hour(value) * 24n;
	},
};
