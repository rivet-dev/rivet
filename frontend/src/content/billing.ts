import type { Rivet } from "@rivet-gg/cloud";
import { bigBytes } from "@/utils/bytes";

const ACTOR_AWAKE_PRICE_PER_SECOND =
	0.05 /
	1000 /* $0.05 per 1k hours */ /
	60 /* minutes per hour */ /
	60 /* seconds per minute */; // ≈ 0.00000001389 dollars per actor-second

/**
 * prices:
 * actor_awake: $0.05 per 1k awake actor-hours
 * kv_storage_used: $0.40 per GB-month
 * kv_read: $0.20 per million 4KB units
 * kv_write: $1 per million 4KB units
 * gateway_egress: $0.15 per GB
 *
 * measurement units:
 * actor_awake measured in seconds
 * kv_storage_used measured in bytes
 * kv_read measured in bytes (rounded up to 4KB units)
 * kv_write measured in bytes (rounded up to 4KB units)
 * gateway_egress measured in bytes
 *
 * included in plans:
 * free:
 *   actor_awake: $5 dollars worth
 *   kv_storage_used: 5 GB
 *   kv_read: 200M
 *   kv_write: 5M
 *   gateway_egress: 100 GB
 * pro:
 *   actor_awake: $20 dollars worth
 *   kv_storage_used: 5 GB
 *   kv_read: 25B
 *   kv_write: 50M
 *   gateway_egress: 1 TB
 * team:
 *   actor_awake: $20 dollars worth
 *   kv_storage_used: 5 GB
 *   kv_read: 25B
 *   kv_write: 50M
 *   gateway_egress: 1 TB
 */

type BilledMetrics = Extract<
	Rivet.MetricName,
	| "actor_awake"
	| "kv_storage_used"
	| "kv_read"
	| "kv_write"
	| "gateway_egress"
>;
export const BILLING = {
	included: {
		free: {
			kv_read: 200_000_000n * bigBytes.KiB(4n), // 200M 4KB units
			kv_write: 5_000_000n * bigBytes.KiB(4n), // 5M 4KB units
			gateway_egress: bigBytes.GiB(100n), // 100 GB
			kv_storage_used: bigBytes.GiB(5n), // 5 GB
			actor_awake: BigInt(
				5_00 /* $5 to cents */ / ACTOR_AWAKE_PRICE_PER_SECOND,
			),
		},
		pro: {
			kv_read: 25_000_000_000n * bigBytes.KiB(4n), // 25B 4KB units
			kv_write: 50_000_000n * bigBytes.KiB(4n), // 50M 4KB units
			gateway_egress: bigBytes.TiB(1n), // 1 TB
			kv_storage_used: bigBytes.GiB(5n), // 5 GB
			actor_awake: BigInt(
				20_00 /* $20 to cents */ / ACTOR_AWAKE_PRICE_PER_SECOND,
			),
		},
		get team() {
			return this.pro;
		},
	} satisfies Record<
		Rivet.BillingPlan,
		Partial<Record<BilledMetrics, bigint>>
	>,
	prices: {
		actor_awake: 14n,
		kv_storage_used: 400n,
		kv_read: 49n,
		kv_write: 244n,
		gateway_egress: 150n,
	} satisfies Record<BilledMetrics, bigint>,
};
