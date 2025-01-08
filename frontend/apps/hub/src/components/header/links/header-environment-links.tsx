import {
	environmentByIdQueryOptions,
	projectByIdQueryOptions,
	projectEnvironmentQueryOptions,
	projectMetadataQueryOptions,
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
	faPuzzlePiece,
} from "@rivet-gg/icons";
import { useSuspenseQueries, useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
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
			projectMetadataQueryOptions({ projectId, environmentId }),
		],
	});

	return (
		<>
			<HeaderLink icon={faActorsBorderless}>
				<Link
					to="/projects/$projectNameId/environments/$environmentNameId/actors"
					params={{ projectNameId, environmentNameId }}
				>
					Actors
				</Link>
			</HeaderLink>
			<HeaderLink icon={faHammer}>
				<Link
					to="/projects/$projectNameId/environments/$environmentNameId/builds"
					params={{ projectNameId, environmentNameId }}
				>
					Builds
				</Link>
			</HeaderLink>
			{backendModulesEnabled ? (
				<GuardEnterprise>
					<HeaderLink icon={faPuzzle}>
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/backend"
							params={{ projectNameId, environmentNameId }}
						>
							Backend
						</Link>
					</HeaderLink>
					<HeaderLink icon={faPuzzlePiece}>
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/modules"
							params={{ projectNameId, environmentNameId }}
						>
							Modules
						</Link>
					</HeaderLink>
				</GuardEnterprise>
			) : null}
			{legacyLobbiesEnabled ? (
				<>
					<HeaderLink icon={faCodeBranch}>
						<Link
							to="/projects/$projectNameId/environments/$environmentNameId/versions"
							params={{ projectNameId, environmentNameId }}
						>
							Versions
						</Link>
					</HeaderLink>
					{data.namespace.config.matchmaker ? (
						<HeaderLink icon={faChessKnight}>
							<Link
								to="/projects/$projectNameId/environments/$environmentNameId/lobbies"
								params={{ projectNameId, environmentNameId }}
							>
								Lobbies
							</Link>
						</HeaderLink>
					) : null}
					{data.namespace.config.cdn ? (
						<HeaderLink icon={faGlobe}>
							<Link
								to="/projects/$projectNameId/environments/$environmentNameId/cdn"
								params={{ projectNameId, environmentNameId }}
							>
								CDN
							</Link>
						</HeaderLink>
					) : null}
				</>
			) : null}
			<HeaderLink icon={faKey}>
				<Link
					to="/projects/$projectNameId/environments/$environmentNameId/tokens"
					params={{ projectNameId, environmentNameId }}
				>
					Tokens
				</Link>
			</HeaderLink>
		</>
	);
}
