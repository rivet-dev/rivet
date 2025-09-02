import { useSuspenseQuery } from "@tanstack/react-query";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	Flex,
} from "@/components";
import * as MatchmakerLobbyConfigForm from "@/domains/project/forms/matchmaker-lobby-config-form";
import { useMatchmakerLobbyConfigFormHandler } from "../hooks/use-matchmaker-lobby-config-form-handler";
import { projectEnvironmentQueryOptions } from "../queries";

interface MatchMakerLobbyConfigSettingsCardProps {
	projectId: string;
	environmentId: string;
}

export function MatchMakerLobbyConfigSettingsCard({
	environmentId,
	projectId,
}: MatchMakerLobbyConfigSettingsCardProps) {
	const { data } = useSuspenseQuery(
		projectEnvironmentQueryOptions({ projectId, environmentId }),
	);

	const handleSubmit = useMatchmakerLobbyConfigFormHandler({
		environmentId,
		projectId,
	});

	return (
		<MatchmakerLobbyConfigForm.Form
			onSubmit={handleSubmit}
			defaultValues={{
				maxPlayers:
					data.namespace.config.matchmaker.maxPlayersPerClient,
				lobbyCountMax: data.namespace.config.matchmaker.lobbyCountMax,
			}}
		>
			<Card>
				<CardHeader>
					<CardTitle>Config</CardTitle>
				</CardHeader>
				<CardContent>
					<Flex gap="4" direction="col">
						<MatchmakerLobbyConfigForm.LobbyCountMax />
						<MatchmakerLobbyConfigForm.MaxPlayers />
					</Flex>
				</CardContent>
				<CardFooter>
					<MatchmakerLobbyConfigForm.Submit>
						Save
					</MatchmakerLobbyConfigForm.Submit>
				</CardFooter>
			</Card>
		</MatchmakerLobbyConfigForm.Form>
	);
}
