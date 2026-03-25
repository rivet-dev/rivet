import { forwardRef, useEffect, useMemo, useState } from "react";

interface RelativeTimeProps {
	time: Date;
}

const relativeTimeFormat = new Intl.RelativeTimeFormat("en", {
	numeric: "auto",
	style: "narrow",
});

function decompose(duration: number) {
	const milliseconds = duration % 1000;
	const seconds = Math.floor(duration / 1000);
	const minutes = Math.floor(seconds / 60);
	const hours = Math.floor(minutes / 60);
	const days = Math.floor(hours / 24);
	const years = Math.floor(days / 365);
	return { years, days, hours, minutes, seconds, milliseconds };
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
			const { years, days, hours, minutes, seconds } =
				decompose(duration);

			if (Math.abs(years) > 0) {
				return relativeTimeFormat.format(-years, "years");
			}
			if (Math.abs(days) > 0) {
				return relativeTimeFormat.format(-days, "days");
			}
			if (Math.abs(hours) > 0) {
				return relativeTimeFormat.format(-hours, "hours");
			}
			if (Math.abs(minutes) > 0) {
				return relativeTimeFormat.format(-minutes, "minutes");
			}
			return relativeTimeFormat.format(-seconds, "seconds");
		}, [now, time]);

		return (
			<time ref={ref} {...props}>
				{value}
			</time>
		);
	},
);
