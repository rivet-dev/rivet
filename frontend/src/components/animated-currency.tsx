"use client";
import {
	animate,
	domAnimation,
	LazyMotion,
	motion,
	useMotionValue,
	useTransform,
} from "framer-motion";
import { useEffect } from "react";
import { formatCurrency } from "./lib/formatter";

interface AnimatedCurrencyProps {
	value: number;
}

export function AnimatedCurrency({ value }: AnimatedCurrencyProps) {
	const startValue = useMotionValue(0);
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
