'use client';

import { motion, useReducedMotion } from 'framer-motion';

// Pixel-grid bar chart for dark data cards: each column is a stack of rounded
// square cells filled bottom-up. Designed to live inside an InkPanel; filled
// cells are warm white with an optional sage accent column. Animates a
// bottom-up stagger fill on scroll into view, or renders static when the
// user prefers reduced motion or `animate` is false.

interface PixelGridChartProps {
	values: number[];
	rows?: number;
	cell?: number;
	gap?: number;
	accentColumn?: number;
	accentColor?: string;
	animate?: boolean;
	className?: string;
}

export const PixelGridChart = ({
	values,
	rows = 11,
	cell = 13,
	gap = 5,
	accentColumn,
	accentColor = '#93A286',
	animate = true,
	className,
}: PixelGridChartProps) => {
	const reducedMotion = useReducedMotion();
	const animated = animate && !reducedMotion;

	const width = values.length * (cell + gap) - gap;
	const height = rows * (cell + gap) - gap;
	const radius = cell * 0.22;

	const cells: Array<{ x: number; y: number; fill: string; delay: number }> = [];
	values.forEach((value, col) => {
		const filled = Math.round(Math.max(0, Math.min(1, value)) * rows);
		for (let row = 0; row < rows; row++) {
			const isFilled = row < filled;
			cells.push({
				x: col * (cell + gap),
				y: height - (row + 1) * (cell + gap) + gap,
				fill: isFilled
					? col === accentColumn
						? accentColor
						: '#F4EFE6'
					: 'rgba(244, 239, 230, 0.07)',
				// Filled cells stagger in by column then row; empty cells are static.
				delay: isFilled ? col * 0.05 + row * 0.02 : 0,
			});
		}
	});

	return (
		<svg
			viewBox={`0 0 ${width} ${height}`}
			width="100%"
			role="img"
			aria-hidden="true"
			focusable="false"
			className={className}
		>
			{cells.map((c, i) =>
				animated && c.delay > 0 ? (
					<motion.rect
						key={i}
						x={c.x}
						y={c.y}
						width={cell}
						height={cell}
						rx={radius}
						fill={c.fill}
						initial={{ opacity: 0, scale: 0.85 }}
						whileInView={{ opacity: 1, scale: 1 }}
						viewport={{ once: true, margin: '-10% 0px' }}
						transition={{ duration: 0.25, delay: c.delay }}
					/>
				) : (
					<rect key={i} x={c.x} y={c.y} width={cell} height={cell} rx={radius} fill={c.fill} />
				),
			)}
		</svg>
	);
};
