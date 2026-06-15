import { useCallback, useEffect, useRef } from "react";

export function useTimeout(
	callback: () => void,
	delay: number | null | undefined,
) {
	const callbackRef = useRef(callback);
	const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	useEffect(() => {
		callbackRef.current = callback;
	}, [callback]);

	const stop = useCallback(() => {
		if (timeoutRef.current !== null) {
			clearTimeout(timeoutRef.current);
			timeoutRef.current = null;
		}
	}, []);

	const start = useCallback(() => {
		if (delay == null) return;
		if (timeoutRef.current !== null) {
			clearTimeout(timeoutRef.current);
		}
		timeoutRef.current = setTimeout(() => {
			timeoutRef.current = null;
			callbackRef.current();
		}, delay);
	}, [delay]);

	const reset = useCallback(() => {
		stop();
		start();
	}, [stop, start]);

	useEffect(() => stop, [stop]);

	return { reset, start, stop };
}
