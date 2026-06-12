import type { ReactNode } from 'react';
import type { ComparisonStatus } from '@/data/compare/types';

const STATUS_STYLES: Record<ComparisonStatus, { dot: string; label: string }> = {
	yes: { dot: 'bg-emerald-400/90', label: 'Yes' },
	partial: { dot: 'bg-amber-400/80', label: 'Partial' },
	'coming-soon': { dot: 'bg-violet-400/80', label: 'Coming soon' },
	no: { dot: 'bg-zinc-700', label: 'No' },
};

interface FeatureStatusProps {
	status: ComparisonStatus;
	text: ReactNode;
}

export function FeatureStatus({ status, text }: FeatureStatusProps) {
	const { dot, label } = STATUS_STYLES[status];
	return (
		<div className="flex items-start gap-2.5">
			<span
				className={`mt-[7px] h-1.5 w-1.5 flex-shrink-0 rounded-full ${dot}`}
				title={label}
			/>
			<div className="min-w-0">{text}</div>
		</div>
	);
}
