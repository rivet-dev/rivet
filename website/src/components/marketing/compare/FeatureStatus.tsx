import {
	Icon,
	faCheck,
	faHourglass,
	faMinus,
	faXmark,
} from '@rivet-gg/icons';
import type { ReactNode } from 'react';
import type { ComparisonStatus } from '@/data/compare/types';

interface FeatureStatusProps {
	status: ComparisonStatus;
	text: ReactNode;
}

export function FeatureStatus({ status, text }: FeatureStatusProps) {
	let icon, bgColor, textColor;

	switch (status) {
		case 'yes':
			icon = faCheck;
			bgColor = 'bg-green-500/20';
			textColor = 'text-green-500';
			break;
		case 'no':
			icon = faXmark;
			bgColor = 'bg-red-500/20';
			textColor = 'text-red-500';
			break;
		case 'partial':
			icon = faMinus;
			bgColor = 'bg-amber-500/20';
			textColor = 'text-amber-500';
			break;
		case 'coming-soon':
			icon = faHourglass;
			bgColor = 'bg-purple-500/20';
			textColor = 'text-purple-500';
			break;
	}

	return (
		<div className="flex items-start">
			<div
				className={`flex-shrink-0 w-5 h-5 rounded-full ${bgColor} flex items-center justify-center ${textColor} mr-2 mt-0.5`}
			>
				<Icon icon={icon} className="text-xs" />
			</div>
			<div>{text}</div>
		</div>
	);
}
