import {
	Button,
	CopyArea,
	CopyButton,
	Flex,
	Skeleton,
	WithTooltip,
} from "@rivet-gg/components";
import type { PropsWithChildren, ReactNode } from "react";
import {
	type LobbySummary as LobbySummaryType,
	useEnvironmentMatchmakeDeleteLobbyMutation,
} from "../../queries";
import { LobbyRegion } from "./lobby-region";

function Container({ children }: PropsWithChildren) {
	return (
		<Flex gap="4" direction="col" pt="4" px="4">
			{children}
		</Flex>
	);
}

interface LobbySummaryProps extends LobbySummaryType {
	isLive?: boolean;
	projectId: string;
	rightSide?: ReactNode;
}

export function LobbySummary({
	lobbyGroupNameId,
	lobbyId,
	projectId,
	regionId,
	rightSide,
	namespaceId: environmentId,
	isLive,
}: LobbySummaryProps) {
	const { mutate: deleteLobby } =
		useEnvironmentMatchmakeDeleteLobbyMutation();

	return (
		<Container>
			<Flex
				gap="2"
				direction={{ initial: "col", md: "row" }}
				className="flex-wrap"
			>
				<Flex direction="col" gap="4" className="min-w-0" items="start">
					<Flex direction="col" gap="4" items="start">
						<Flex gap="2">
							<WithTooltip
								trigger={
									<CopyArea
										className="min-w-0 truncate"
										variant="discrete"
										value={lobbyGroupNameId}
									/>
								}
								content="Lobby Group Name"
							/>
							<Button asChild variant="outline">
								<div>
									<LobbyRegion
										projectId={projectId}
										regionId={regionId}
										showLabel
									/>
								</div>
							</Button>
						</Flex>

						<Flex gap="2">
							<CopyButton value={lobbyId}>
								<Button variant="outline">Copy ID</Button>
							</CopyButton>
							{isLive ? (
								<Button
									variant="destructive"
									onClick={() =>
										deleteLobby({
											lobbyId,
											environmentId,
											projectId,
										})
									}
								>
									Destroy
								</Button>
							) : null}
						</Flex>
					</Flex>
				</Flex>
				<Flex
					direction="col"
					gap="4"
					className="flex-1"
					items={{ initial: "start", md: "end" }}
				>
					{rightSide}
				</Flex>
			</Flex>
		</Container>
	);
}

LobbySummary.Skeleton = function LobbyLogsSummarySkeleton() {
	return (
		<Container>
			<div className="flex gap-2 items-center flex-wrap">
				<Skeleton className="mt-1 h-6 w-1/4" />
				<Skeleton className="mt-1 h-6 w-1/4" />
				<Skeleton className="mt-1 h-6 w-1/4" />
				<Skeleton className="mt-1 h-6 w-1/5" />
			</div>

			<Skeleton className="mt-3 mx-auto h-10 w-3/4" />
		</Container>
	);
};
