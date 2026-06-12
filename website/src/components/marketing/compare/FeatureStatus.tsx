import type { ReactNode } from 'react';
import type { ComparisonStatus } from '@/data/compare/types';

// Printed-catalog status marks: a pine check for yes, a hollow circle for
// partial, a faint dash for no, and a dotted circle for coming soon. Pure
// inline SVG so the component stays hook-free for zero-JS pages.
const STATUS_MARKS: Record<ComparisonStatus, { icon: ReactNode; label: string }> = {
	yes: {
		label: 'Yes',
		icon: (
			<svg viewBox="0 0 12 12" aria-hidden="true" className="h-3 w-3 text-pine">
				<path
					d="M2 6.5 4.8 9.3 10 3.2"
					fill="none"
					stroke="currentColor"
					strokeWidth="1.6"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
			</svg>
		),
	},
	partial: {
		label: 'Partial',
		icon: (
			<svg viewBox="0 0 12 12" aria-hidden="true" className="h-3 w-3 text-ink-faint">
				<circle cx="6" cy="6" r="4.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
				<path d="M6 1.5A4.5 4.5 0 0 1 6 10.5Z" fill="currentColor" stroke="none" />
			</svg>
		),
	},
	'coming-soon': {
		label: 'Coming soon',
		icon: (
			<svg viewBox="0 0 12 12" aria-hidden="true" className="h-3 w-3 text-ink-faint">
				<circle
					cx="6"
					cy="6"
					r="4.5"
					fill="none"
					stroke="currentColor"
					strokeWidth="1.4"
					strokeDasharray="2 2.4"
					strokeLinecap="round"
				/>
			</svg>
		),
	},
	no: {
		label: 'No',
		icon: (
			<svg viewBox="0 0 12 12" aria-hidden="true" className="h-3 w-3 text-ink/25">
				<path d="M2.5 6h7" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
			</svg>
		),
	},
};

interface FeatureStatusProps {
	status: ComparisonStatus;
	text: ReactNode;
}

export function FeatureStatus({ status, text }: FeatureStatusProps) {
	const { icon, label } = STATUS_MARKS[status];
	return (
		<div className="flex items-start gap-2.5">
			<span className="mt-[4px] flex-shrink-0" title={label}>
				{icon}
			</span>
			<div className="min-w-0">{text}</div>
		</div>
	);
}
