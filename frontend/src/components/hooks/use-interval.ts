import { useCallback, useEffect, useRef } from "react";

export function useInterval(
	callback: () => void,
	delay: number | null | undefined,
) {
	const callbackRef = useRef(callback);
	const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

	useEffect(() => {
		callbackRef.current = callback;
	}, [callback]);

	const start = useCallback(() => {
		if (delay == null) return;
		if (intervalRef.current !== null) {
			clearInterval(intervalRef.current);
		}
		intervalRef.current = setInterval(() => callbackRef.current(), delay);
	}, [delay]);

	const stop = useCallback(() => {
		if (intervalRef.current !== null) {
			clearInterval(intervalRef.current);
			intervalRef.current = null;
		}
	}, []);

	const reset = useCallback(() => {
		stop();
		start();
	}, [stop, start]);

	useEffect(() => {
		start();
		return stop;
	}, [start, stop]);

	return { reset, start, stop };
}
