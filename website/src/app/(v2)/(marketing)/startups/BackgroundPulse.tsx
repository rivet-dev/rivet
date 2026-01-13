'use client';

import { motion } from 'framer-motion';

export function BackgroundPulse() {
	return (
		<motion.div
			animate={{ opacity: [0.3, 0.5, 0.3] }}
			transition={{ duration: 4, repeat: Infinity }}
			className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-[#FF4500]/10 via-transparent to-transparent opacity-50"
		/>
	);
}
