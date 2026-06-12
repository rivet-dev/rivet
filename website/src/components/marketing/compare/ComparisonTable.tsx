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
		<div className="overflow-x-auto border-t border-white/10">
			<table className="w-full border-collapse [&_a]:underline [&_a]:text-white [&_a]:hover:text-zinc-300">
				<thead>
					<tr className="border-b border-white/10">
						<th className="py-3 px-4 text-left text-xs font-medium uppercase tracking-wider text-zinc-500">
							Feature
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium">
							<div className="flex items-center gap-2">
								<Icon icon={faRivet} className="text-zinc-500" />
								<span className="text-white">{rivetProductName}</span>
							</div>
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium">
							<div className="flex items-center gap-2">
								{competitorIcon && <Icon icon={competitorIcon} className="text-zinc-500" />}
								<span className="text-zinc-500">{competitorName}</span>
							</div>
						</th>
						<th className="py-3 px-4 text-left text-xs font-medium uppercase tracking-wider text-zinc-500">
							Why it matters
						</th>
					</tr>
				</thead>
				<tbody>
					{featureGroups.map((group) => (
						<React.Fragment key={group.title}>
							<tr className="bg-zinc-900/50 border-b border-white/10">
								<td colSpan={4} className="py-2 px-4 text-sm font-medium text-white">
									{group.title}
								</td>
							</tr>
							{group.rows.map((row) => (
								<tr
									key={row.feature}
									className="border-b border-white/5 hover:bg-white/5 transition-colors"
								>
									<td className="py-3 px-4 text-sm text-white">{row.feature}</td>
									<td className="py-3 px-4 text-sm text-zinc-400">
										<FeatureStatus status={row.rivet.status} text={row.rivet.text} />
									</td>
									<td className="py-3 px-4 text-sm text-zinc-500">
										<FeatureStatus
											status={row.competitor.status}
											text={row.competitor.text}
										/>
									</td>
									<td className="py-3 px-4 text-sm text-zinc-500">{row.importance}</td>
								</tr>
							))}
						</React.Fragment>
					))}
				</tbody>
			</table>
		</div>
	);
}
