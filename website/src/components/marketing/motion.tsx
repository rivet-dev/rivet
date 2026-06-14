'use client';

import { motion, useReducedMotion } from 'framer-motion';
import type { ReactNode } from 'react';

// The marketing motion vocabulary. Every scroll reveal on marketing pages
// goes through Reveal or RevealGroup so timing stays consistent site-wide.

export const EASE = [0.22, 1, 0.36, 1] as const;

export const DURATION = {
	fast: 0.25,
	base: 0.55,
	slow: 0.8,
} as const;

export const RISE = 14;

export const VIEWPORT = { once: true, margin: '-10% 0px' } as const;

interface RevealProps {
	children: ReactNode;
	delay?: number;
	className?: string;
}

// Fade-rise on scroll into view. Degrades to a quick opacity fade under
// reduced motion.
export const Reveal = ({ children, delay = 0, className }: RevealProps) => {
	const reducedMotion = useReducedMotion();
	return (
		<motion.div
			className={className}
			initial={reducedMotion ? { opacity: 0 } : { opacity: 0, y: RISE }}
			whileInView={reducedMotion ? { opacity: 1 } : { opacity: 1, y: 0 }}
			viewport={VIEWPORT}
			transition={{
				duration: reducedMotion ? 0.2 : DURATION.base,
				delay,
				ease: [...EASE],
			}}
		>
			{children}
		</motion.div>
	);
};

interface RevealGroupProps {
	children: ReactNode;
	stagger?: number;
	className?: string;
}

// Container that staggers its direct RevealChild children.
export const RevealGroup = ({ children, stagger = 0.07, className }: RevealGroupProps) => (
	<motion.div
		className={className}
		initial="hidden"
		whileInView="visible"
		viewport={VIEWPORT}
		variants={{
			hidden: {},
			visible: { transition: { staggerChildren: stagger } },
		}}
	>
		{children}
	</motion.div>
);

export const RevealChild = ({ children, className }: { children: ReactNode; className?: string }) => {
	const reducedMotion = useReducedMotion();
	return (
		<motion.div
			className={className}
			variants={{
				hidden: reducedMotion ? { opacity: 0 } : { opacity: 0, y: RISE },
				visible: {
					opacity: 1,
					y: 0,
					transition: { duration: reducedMotion ? 0.2 : DURATION.base, ease: [...EASE] },
				},
			}}
		>
			{children}
		</motion.div>
	);
};
