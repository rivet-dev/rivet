import { forwardRef, useEffect, useMemo, useState } from "react";

interface RelativeTimeProps {
	time: Date;
}

const relativeTimeFormat = new Intl.RelativeTimeFormat("en", {
	numeric: "always",
	style: "narrow",
});

// Only used for sub-minute values, where "always" renders 0 seconds as "in 0s".
const nowFormat = new Intl.RelativeTimeFormat("en", {
	numeric: "auto",
	style: "narrow",
});

function decompose(duration: number) {
	const milliseconds = duration % 1000;
	const seconds = Math.floor(duration / 1000);
	const minutes = Math.floor(seconds / 60);
	const hours = Math.floor(minutes / 60);
	const days = Math.floor(hours / 24);
	const weeks = Math.floor(days / 7);
	const months = Math.floor(days / 30.44);
	const years = Math.floor(days / 365.25);
	return {
		years,
		months,
		weeks,
		days,
		hours,
		minutes,
		seconds,
		milliseconds,
	};
}

// Shared per-tier clock. Each tier has one global interval shared by all
// subscribers, so N mounted <RelativeTime> components only create at most 4
// intervals total rather than one per component.
type Listener = (now: number) => void;

const tiers = [1_000, 10_000, 60_000, 60_000 * 60] as const;
type Tier = (typeof tiers)[number];

const subscribers = new Map<Tier, Set<Listener>>();
const intervals = new Map<Tier, ReturnType<typeof setInterval>>();

function subscribe(tier: Tier, listener: Listener) {
	let set = subscribers.get(tier);
	if (!set) {
		set = new Set();
		subscribers.set(tier, set);
	}
	set.add(listener);

	if (!intervals.has(tier)) {
		intervals.set(
			tier,
			setInterval(() => {
				const now = Date.now();
				for (const l of subscribers.get(tier) ?? []) l(now);
			}, tier),
		);
	}

	return () => {
		set.delete(listener);
		if (set.size === 0) {
			clearInterval(intervals.get(tier));
			intervals.delete(tier);
			subscribers.delete(tier);
		}
	};
}

function getTier(duration: number): Tier {
	const { days, hours, minutes } = decompose(Math.abs(duration));
	if (days > 0) return 60_000 * 60;
	if (hours > 0) return 60_000;
	if (minutes > 0) return 10_000;
	return 1_000;
}

function useNow(tier: Tier) {
	const [now, setNow] = useState(() => Date.now());
	useEffect(() => subscribe(tier, setNow), [tier]);
	return now;
}

export const RelativeTime = forwardRef<HTMLTimeElement, RelativeTimeProps>(
	({ time, ...props }, ref) => {
		const tier = getTier(Date.now() - time.getTime());
		const now = useNow(tier);

		const value = useMemo(() => {
			const duration = now - time.getTime();
			// Negative durations are future timestamps, formatted as "in 2m".
			const direction = duration < 0 ? 1 : -1;
			const { years, months, weeks, days, hours, minutes, seconds } =
				decompose(Math.abs(duration));

			if (years > 0) {
				return relativeTimeFormat.format(direction * years, "years");
			}
			if (months > 0) {
				return relativeTimeFormat.format(direction * months, "months");
			}
			if (weeks > 0) {
				return relativeTimeFormat.format(direction * weeks, "weeks");
			}
			if (days > 0) {
				return relativeTimeFormat.format(direction * days, "days");
			}
			if (hours > 0) {
				return relativeTimeFormat.format(direction * hours, "hours");
			}
			if (minutes > 0) {
				return relativeTimeFormat.format(
					direction * minutes,
					"minutes",
				);
			}
			if (seconds < 1) {
				return nowFormat.format(0, "seconds");
			}
			return relativeTimeFormat.format(direction * seconds, "seconds");
		}, [now, time]);

		return (
			<time ref={ref} {...props}>
				{value}
			</time>
		);
	},
);
