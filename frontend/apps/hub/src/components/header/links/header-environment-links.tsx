import {
	environmentByIdQueryOptions,
	projectByIdQueryOptions,
	projectEnvironmentQueryOptions,
	environmentMetadataQueryOptions,
} from "@/domains/project/queries";
import { GuardEnterprise } from "@/lib/guards";
import {
	faActorsBorderless,
	faChessKnight,
	faCodeBranch,
	faGlobe,
	faHammer,
	faKey,
	faPuzzle,
} from "@rivet-gg/icons";
import { useSuspenseQueries, useSuspenseQuery } from "@tanstack/react-query";
import { HeaderLink } from "../header-link";

interface EnvironmentLinksProps {
	projectNameId: string;
	environmentNameId: string;
}

export function HeaderEnvironmentLinks({
	projectNameId,
	environmentNameId,
}: EnvironmentLinksProps) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));

	const {
		data: { namespaceId: environmentId },
	} = useSuspenseQuery(
		environmentByIdQueryOptions({ projectId, environmentNameId }),
	);

	const [
		{ data },
		{
			data: { legacyLobbiesEnabled, backendModulesEnabled },
		},
	] = useSuspenseQueries({
		queries: [
			projectEnvironmentQueryOptions({ projectId, environmentId }),
			environmentMetadataQueryOptions({ projectId, environmentId }),
		],
	});

	return (
		<>
			<HeaderLink
				icon={faActorsBorderless}
				to="/projects/$projectNameId/environments/$environmentNameId/actors"
				params={{ projectNameId, environmentNameId }}
			>
				Actors
			</HeaderLink>
			<HeaderLink
				icon={faHammer}
				to="/projects/$projectNameId/environments/$environmentNameId/builds"
				params={{ projectNameId, environmentNameId }}
			>
				Builds
			</HeaderLink>
			{backendModulesEnabled ? (
				<GuardEnterprise>
					<HeaderLink
						icon={faPuzzle}
						to="/projects/$projectNameId/environments/$environmentNameId/backend"
						params={{ projectNameId, environmentNameId }}
					>
						Backend
					</HeaderLink>
				</GuardEnterprise>
			) : null}
			{legacyLobbiesEnabled ? (
				<>
					<HeaderLink
						icon={faCodeBranch}
						to="/projects/$projectNameId/environments/$environmentNameId/versions"
						params={{ projectNameId, environmentNameId }}
					>
						Versions
					</HeaderLink>
					{data.namespace.config.matchmaker ? (
						<HeaderLink
							icon={faChessKnight}
							to="/projects/$projectNameId/environments/$environmentNameId/lobbies"
							params={{ projectNameId, environmentNameId }}
						>
							Lobbies
						</HeaderLink>
					) : null}
					{data.namespace.config.cdn ? (
						<HeaderLink
							icon={faGlobe}
							to="/projects/$projectNameId/environments/$environmentNameId/cdn"
							params={{ projectNameId, environmentNameId }}
						>
							CDN
						</HeaderLink>
					) : null}
				</>
			) : null}
			<HeaderLink
				icon={faKey}
				to="/projects/$projectNameId/environments/$environmentNameId/tokens"
				params={{ projectNameId, environmentNameId }}
			>
				Tokens
			</HeaderLink>
		</>
	);
}
