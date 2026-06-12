import { Icon, faRivet } from '@rivet-gg/icons';
import React from 'react';
import type { IconProp } from '@rivet-gg/icons';
import type { FeatureGroup } from '@/data/compare/types';
import { FeatureStatus } from './FeatureStatus';

interface ComparisonTableProps {
	featureGroups: FeatureGroup[];
	competitorName: string;
	competitorIcon?: IconProp;
	rivetProductName: string;
}

export function ComparisonTable({
	featureGroups,
	competitorName,
	competitorIcon,
	rivetProductName,
}: ComparisonTableProps) {
	return (
		<div className="overflow-x-auto">
			<table className="w-full border-collapse [&_a]:underline [&_a]:decoration-zinc-600 [&_a]:underline-offset-2 [&_a]:text-zinc-200 [&_a]:hover:text-white">
				<thead>
					<tr className="border-b border-white/10">
						<th className="w-[18%] py-4 pl-0 pr-4 text-left text-[11px] font-medium uppercase tracking-wider text-zinc-600">
							Feature
						</th>
						<th className="w-[28%] px-4 py-4 text-left">
							<div className="flex items-center gap-2 text-sm font-medium text-white">
								<Icon icon={faRivet} className="h-3.5 w-3.5 text-zinc-500" />
								{rivetProductName}
							</div>
						</th>
						<th className="w-[28%] px-4 py-4 text-left">
							<div className="flex items-center gap-2 text-sm font-medium text-zinc-400">
								{competitorIcon && <Icon icon={competitorIcon} className="h-3.5 w-3.5 text-zinc-600" />}
								{competitorName}
							</div>
						</th>
						<th className="w-[26%] py-4 pl-4 pr-0 text-left text-[11px] font-medium uppercase tracking-wider text-zinc-600">
							Why it matters
						</th>
					</tr>
				</thead>
				<tbody>
					{featureGroups.map((group) => (
						<React.Fragment key={group.title}>
							<tr className="border-b border-white/10">
								<td colSpan={4} className="pb-3 pl-0 pt-12 text-xs font-medium uppercase tracking-wider text-zinc-500">
									{group.title}
								</td>
							</tr>
							{group.rows.map((row) => (
								<tr key={row.feature} className="border-b border-white/[0.06]">
									<td className="py-5 pl-0 pr-4 align-top text-sm font-medium leading-relaxed text-zinc-200">
										{row.feature}
									</td>
									<td className="px-4 py-5 align-top text-sm leading-relaxed text-zinc-300">
										<FeatureStatus status={row.rivet.status} text={row.rivet.text} />
									</td>
									<td className="px-4 py-5 align-top text-sm leading-relaxed text-zinc-500">
										<FeatureStatus status={row.competitor.status} text={row.competitor.text} />
									</td>
									<td className="py-5 pl-4 pr-0 align-top text-sm leading-relaxed text-zinc-500">
										{row.importance}
									</td>
								</tr>
							))}
						</React.Fragment>
					))}
				</tbody>
			</table>
		</div>
	);
}
