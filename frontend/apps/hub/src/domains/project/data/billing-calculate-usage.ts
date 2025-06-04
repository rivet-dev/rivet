import type { Rivet } from "@rivet-gg/api-full";
import { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { millisecondsToMonths } from "@rivet-gg/components";

export const PRICE_MAP = {
	[RivetEe.ee.billing.Plan.Trial]: 0,
	[RivetEe.ee.billing.Plan.Indie]: 20.0,
	[RivetEe.ee.billing.Plan.Studio]: 200.0,
};
const CREDITS_MAP = {
	[RivetEe.ee.billing.Plan.Trial]: 5.0,
	[RivetEe.ee.billing.Plan.Indie]: 48.21,
	[RivetEe.ee.billing.Plan.Studio]: 29.0,
};

export const BILLING_PLANS_CREDITS_VISIBILITY: RivetEe.ee.billing.Plan[] = [
	RivetEe.ee.billing.Plan.Indie,
	RivetEe.ee.billing.Plan.Trial,
];

const FACTOR = 16.07;

export function calculateUsedCredits({
	usage,
	plan,
}: {
	usage: RivetEe.ee.billing.GameUsage | undefined;
	plan: RivetEe.ee.billing.Plan;
}) {
	const totalUptime =
		usage?.regions.reduce((acc, region) => acc + region.uptime, 0) ?? 0;
	const monthsOfUptime = millisecondsToMonths(totalUptime);
	const usedCredits = monthsOfUptime * FACTOR;

	const overage = Math.max(0, usedCredits - CREDITS_MAP[plan]);

	return {
		max: CREDITS_MAP[plan],
		used: usedCredits,
		remaining: CREDITS_MAP[plan] - usedCredits,
		overage,
		total: PRICE_MAP[plan] + overage,
	};
}

// #region actors pricing

const ACTORS_FACTOR = 32.14;
const ACTORS_CREDITS_MAP = {
	[RivetEe.ee.billing.Plan.Trial]: 5.0,
	[RivetEe.ee.billing.Plan.Indie]: 20.0,
	[RivetEe.ee.billing.Plan.Studio]: 200.0,
};
export function calculateUsedActorCredits({
	actors,
	plan,
	startTs,
	endTs,
}: {
	actors: Rivet.actors.Actor[];
	plan: RivetEe.ee.billing.Plan;
	startTs: Date;
	endTs: Date;
}) {
	const totalDuration = actors
		.filter((actor) => {
			return (
				actor.createdAt < endTs &&
				actor.createdAt > startTs &&
				(actor.destroyedAt > startTs || actor.destroyedAt === undefined)
			);
		})
		.reduce((acc, actor) => {
			const start = Math.max(
				actor.createdAt.getTime(),
				startTs.getTime(),
			);
			const end = Math.min(
				actor.destroyedAt?.getTime() || endTs.getTime(),
				endTs.getTime(),
			);
			const duration = (end - start) * 1.1;

			return duration + acc;
		}, 0);

	const expense = totalDuration / 1000 / 60 / 60 / 24 / 30; // convert to months
	const usedCredits = expense * ACTORS_FACTOR;

	const overage = Math.max(0, usedCredits - ACTORS_CREDITS_MAP[plan]);

	return {
		max: ACTORS_CREDITS_MAP[plan],
		used: usedCredits,
		remaining: ACTORS_CREDITS_MAP[plan] - usedCredits,
		overage,
		total: PRICE_MAP[plan] + overage,
	};
}

// #endregion actors pricing
