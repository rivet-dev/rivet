import { useQuery } from "@tanstack/react-query";
import { Flex, WithTooltip } from "@/components";
import {
	getRegionKey,
	REGION_LABEL,
	RegionIcon,
} from "../matchmaker/lobby-region";
import { useManager } from "./manager-context";

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
	const { data: region } = useQuery(
		useManager().regionQueryOptions(regionId),
	);

	if (!regionId || !region) {
		return null;
	}

	const regionKey = getRegionKey(region?.id);

	if (showLabel) {
		return (
			<Flex gap="2" items="center" justify="center" className={className}>
				<RegionIcon region={regionKey} className="w-4 min-w-4" />
				<span data-slot="label">
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
