"use client";

import { motion } from "framer-motion";

interface FadeInProps {
	children: React.ReactNode;
	delay?: number;
	className?: string;
	direction?: "up" | "down" | "left" | "right" | "none";
	duration?: number;
	fullWidth?: boolean;
}

export function FadeIn({
	children,
	delay = 0,
	className = "",
	direction = "up",
	duration = 0.5,
	fullWidth = false,
}: FadeInProps) {
	const variants = {
		hidden: {
			opacity: 0,
			y: direction === "up" ? 20 : direction === "down" ? -20 : 0,
			x: direction === "left" ? 20 : direction === "right" ? -20 : 0,
		},
		visible: {
			opacity: 1,
			y: 0,
			x: 0,
			transition: {
				duration: duration,
				delay: delay,
				ease: [0.25, 0.1, 0.25, 1.0], // Cubic bezier for sleek feel
			},
		},
	};

	return (
		<motion.div
			initial="hidden"
			whileInView="visible"
			viewport={{ once: true, margin: "-50px" }}
			variants={variants}
			className={className}
			style={{ width: fullWidth ? "100%" : "auto" }}
		>
			{children}
		</motion.div>
	);
}

export function FadeInStagger({
	children,
	delay = 0,
	className = "",
	faster = false,
}: { children: React.ReactNode; delay?: number; className?: string; faster?: boolean }) {
	return (
		<motion.div
			initial="hidden"
			whileInView="visible"
			viewport={{ once: true, margin: "-50px" }}
			transition={{ staggerChildren: faster ? 0.05 : 0.1, delayChildren: delay }}
			className={className}
		>
			{children}
		</motion.div>
	);
}

export function FadeInItem({ children, className = "" }: { children: React.ReactNode; className?: string }) {
	const variants = {
		hidden: { opacity: 0, y: 20 },
		visible: {
			opacity: 1,
			y: 0,
			transition: {
				duration: 0.5,
				ease: [0.25, 0.1, 0.25, 1.0],
			},
		},
	};

	return (
		<motion.div variants={variants} className={className}>
			{children}
		</motion.div>
	);
}

