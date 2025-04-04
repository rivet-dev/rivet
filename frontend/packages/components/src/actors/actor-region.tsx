import { Flex, WithTooltip } from "@rivet-gg/components";
import {
	REGION_LABEL,
	RegionIcon,
	getRegionKey,
} from "../matchmaker/lobby-region";
import { actorRegionsAtom, type Actor } from "./actor-context";
import { selectAtom } from "jotai/utils";
import { useAtomValue } from "jotai";
import { useCallback } from "react";

interface ActorRegionProps {
	regionId?: string;
	showLabel?: boolean | "abbreviated";
	className?: string;
}

export function ActorRegion({
	showLabel,
	regionId,
	className,
}: ActorRegionProps) {
	const region = useAtomValue(
		selectAtom(
			actorRegionsAtom,
			useCallback(
				(regions) => regions.find((region) => region.id === regionId),
				[regionId],
			),
		),
	);

	const regionKey = getRegionKey(region?.id);

	if (showLabel) {
		return (
			<Flex gap="2" items="center" justify="center" className={className}>
				<RegionIcon region={regionKey} className="w-4 min-w-4" />
				<span>
					{showLabel === "abbreviated"
						? regionKey.toUpperCase()
						: (REGION_LABEL[regionKey] ?? REGION_LABEL.unknown)}
				</span>
			</Flex>
		);
	}

	return (
		<WithTooltip
			content={REGION_LABEL[regionKey] ?? REGION_LABEL.unknown}
			trigger={
				<Flex
					gap="2"
					items="center"
					justify="center"
					className={className}
				>
					<RegionIcon region={regionKey} className="w-4 min-w-4" />
				</Flex>
			}
		/>
	);
}
