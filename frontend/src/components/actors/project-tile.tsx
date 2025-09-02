import type { Rivet } from "@rivet-gg/api";
import { AssetImage, Flex, Text } from "@/components";
import { BillingPlanBadge } from "./billing/billing-plan-badge";

interface ProjectTileProps
	extends Pick<
		Rivet.game.GameSummary,
		"gameId" | "displayName" | "logoUrl"
	> {}

export function ProjectTile({
	gameId: projectId,
	displayName,
	logoUrl,
}: ProjectTileProps) {
	return (
		<Flex
			className="rounded-md border-2 p-4 hover:bg-accent"
			direction="col"
			justify="center"
			items="center"
		>
			<div>
				<AssetImage
					src={logoUrl || "/games/blank/blankgame.svg"}
					className="w-24 h-24 mx-auto object-contain"
					alt="Project logo"
				/>
			</div>
			<Text className="line-clamp-1 mb-2">{displayName}</Text>
			<BillingPlanBadge projectId={projectId} />
		</Flex>
	);
}
