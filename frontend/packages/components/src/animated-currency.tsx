"use client";
import {
	LazyMotion,
	animate,
	domAnimation,
	motion,
	useMotionValue,
	useTransform,
} from "framer-motion";
import { useEffect } from "react";
import { formatCurrency } from "./lib/formatter";

interface AnimatedCurrencyProps {
	value: number;
	from?: number;
}

export function AnimatedCurrency({ value, from }: AnimatedCurrencyProps) {
	const startValue = useMotionValue(from ?? 0);
	const currentValue = useTransform(startValue, (v) => formatCurrency(v));

	useEffect(() => {
		animate(startValue, value, { duration: 1, ease: "circIn" });
	}, [startValue, value]);

	return (
		<LazyMotion features={domAnimation}>
			<motion.span>{currentValue}</motion.span>
		</LazyMotion>
	);
}
