export const fileSize = {
	bytes(v: number) {
		return Math.floor(v);
	},

	kilobytes(v: number) {
		return this.bytes(v * 1000);
	},
	megabytes(v: number) {
		return this.bytes(v * 1000 * 1000);
	},
	gigabytes(v: number) {
		return this.bytes(v * 1000 * 1000 * 1000);
	},

	kibibytes(v: number) {
		return this.bytes(v * 1024);
	},
	mebibytes(v: number) {
		return this.bytes(v * 1024 * 1024);
	},
	gibibytes(v: number) {
		return this.bytes(v * 1024 * 1024 * 1024);
	},
};
