import { useQuery } from "@tanstack/react-query";
import { Flex, WithTooltip } from "@/components";
import {
	getRegionKey,
	getRegionLabel,
	REGION_LABEL,
	RegionIcon,
} from "../matchmaker/lobby-region";
import { useDataProvider } from "./data-provider";

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
		useDataProvider().datacenterQueryOptions(regionId),
	);

	if (!regionId || !region) {
		return null;
	}

	const regionKey = getRegionKey(region?.name);

	if (showLabel) {
		return (
			<Flex gap="2" items="center" justify="center" className={className}>
				<RegionIcon region={regionKey} className="w-4 min-w-4" />
				<span data-slot="label">
					{showLabel === "abbreviated"
						? regionKey?.toUpperCase()
						: getRegionLabel(regionKey)}
				</span>
			</Flex>
		);
	}

	return (
		<WithTooltip
			content={getRegionLabel(regionKey)}
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
