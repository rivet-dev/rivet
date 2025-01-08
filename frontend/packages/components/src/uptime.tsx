"use client";
import { useEffect, useState } from "react";
import { type DurationOptions, formatDuration } from "./lib/formatter";

interface UptimeProps extends DurationOptions {
	createTs: Date;
}

export function Uptime({ createTs, ...options }: UptimeProps) {
	const [uptime, setUptime] = useState(() => Date.now() - createTs.getTime());

	useEffect(() => {
		const interval = setInterval(() => {
			setUptime(Date.now() - createTs.getTime());
		}, 1000);
		return () => {
			clearInterval(interval);
		};
	}, [createTs]);

	return <>{formatDuration(uptime, options)}</>;
}
