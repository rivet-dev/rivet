import { forwardRef, useMemo } from "react";

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

export const RelativeTime = forwardRef<HTMLTimeElement, RelativeTimeProps>(
	({ time, ...props }, ref) => {
		const value = useMemo(() => {
			const duration = Date.now() - time.getTime();
			const { years, days, hours, minutes, seconds } =
				decompose(duration);

			if (years > 0) {
				return relativeTimeFormat.format(-years, "years");
			}
			if (days > 0) {
				return relativeTimeFormat.format(-days, "days");
			}
			if (hours > 0) {
				return relativeTimeFormat.format(-hours, "hours");
			}
			if (minutes > 0) {
				return relativeTimeFormat.format(-minutes, "minutes");
			}
			return relativeTimeFormat.format(-seconds, "seconds");
		}, [time]);

		return (
			<time ref={ref} {...props}>
				{value}
			</time>
		);
	},
);
