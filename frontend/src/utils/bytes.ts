const KiB = 1024;
const MiB = KiB * 1024;
const GiB = MiB * 1024;
const TiB = GiB * 1024;

export const bytes = {
	KiB(value: number): number {
		return value * KiB;
	},
	MiB(value: number): number {
		return value * MiB;
	},
	GiB(value: number): number {
		return value * GiB;
	},
	TiB(value: number): number {
		return value * TiB;
	},
};

export const bigBytes = {
	KiB(value: bigint): bigint {
		return value * BigInt(KiB);
	},
	MiB(value: bigint): bigint {
		return value * BigInt(MiB);
	},
	GiB(value: bigint): bigint {
		return value * BigInt(GiB);
	},
	TiB(value: bigint): bigint {
		return value * BigInt(TiB);
	},
};

export function formatBytes(value: number): string {
	if (value >= TiB) return `${(value / TiB).toFixed(2)} TB`;
	if (value >= GiB) return `${(value / GiB).toFixed(2)} GB`;
	if (value >= MiB) return `${(value / MiB).toFixed(2)} MB`;
	if (value >= KiB) return `${(value / KiB).toFixed(2)} KB`;
	return `${value} B`;
}
