import {
	environmentByIdQueryOptions,
	projectByIdQueryOptions,
	projectEnvironmentQueryOptions,
	projectMetadataQueryOptions,
	projectQueryOptions,
} from "@/domains/project/queries";
import { GuardEnterprise } from "@/lib/guards";
import { Badge, CommandGroup, CommandItem } from "@rivet-gg/components";
import {
	Icon,
	faActorsBorderless,
	faCodeBranch,
	faGear,
	faGlobe,
	faHammer,
	faJoystick,
	faKey,
	faLink,
	faPuzzle,
	faScroll,
	faUserCog,
} from "@rivet-gg/icons";
import { useSuspenseQueries, useSuspenseQuery } from "@tanstack/react-query";
import { useCommandPanelNavigation } from "../command-panel-navigation-provider";

interface EnvironmentCommandPanelPage {
	projectNameId: string;
	environmentNameId: string;
}

export function EnvironmentCommandPanelPage({
	projectNameId,
	environmentNameId,
}: EnvironmentCommandPanelPage) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));
	const {
		data: { namespaceId: environmentId },
	} = useSuspenseQuery(
		environmentByIdQueryOptions({ projectId, environmentNameId }),
	);

	const [
		{
			data: { displayName, versions },
		},
		{
			data: { legacyLobbiesEnabled, backendModulesEnabled },
		},
	] = useSuspenseQueries({
		queries: [
			projectQueryOptions(projectId),
			projectMetadataQueryOptions({ projectId, environmentId }),
		],
	});

	const {
		data: {
			namespace: { versionId, config },
		},
	} = useSuspenseQuery(
		projectEnvironmentQueryOptions({ environmentId, projectId }),
	);

	const { navigate } = useCommandPanelNavigation();

	const currentVersion = versions.find(
		(version) => version.versionId === versionId,
	);

	return (
		<>
			<CommandGroup heading={displayName}>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/environments/$environmentNameId/actors",
							params: { projectNameId, environmentNameId },
						});
					}}
				>
					<Icon icon={faActorsBorderless} />
					Actors
				</CommandItem>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/environments/$environmentNameId/builds",
							params: { projectNameId, environmentNameId },
						});
					}}
				>
					<Icon icon={faHammer} />
					Builds
				</CommandItem>
				{backendModulesEnabled ? (
					<GuardEnterprise>
						<CommandItem
							onSelect={() => {
								navigate({
									to: "/projects/$projectNameId/environments/$environmentNameId/backend",
									params: {
										projectNameId,
										environmentNameId,
									},
								});
							}}
						>
							<Icon icon={faPuzzle} />
							Backend
						</CommandItem>
					</GuardEnterprise>
				) : null}

				{legacyLobbiesEnabled ? (
					<CommandItem
						onSelect={() => {
							navigate({
								to: "/projects/$projectNameId/environments/$environmentNameId/versions",
								params: { projectNameId, environmentNameId },
							});
						}}
					>
						<Icon icon={faCodeBranch} />
						Versions
						{currentVersion ? (
							<Badge className="ml-2">
								{currentVersion?.displayName}
							</Badge>
						) : null}
					</CommandItem>
				) : null}
			</CommandGroup>
			{legacyLobbiesEnabled ? (
				<>
					{config.cdn ? (
						<CommandGroup heading="CDN">
							<CommandItem
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/cdn",
										params: {
											projectNameId,
											environmentNameId,
										},
									});
								}}
							>
								<Icon icon={faGlobe} />
								CDN Overview
							</CommandItem>
							<CommandItem
								keywords={["cdn", "auth", "users"]}
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/cdn",
										params: {
											projectNameId,
											environmentNameId,
										},
										search: { modal: "cdn-users" },
									});
								}}
							>
								<Icon icon={faUserCog} />
								Manage authenticated users
							</CommandItem>
							<CommandItem
								keywords={["cdn", "custom", "domains"]}
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/cdn",
										params: {
											projectNameId,
											environmentNameId,
										},
										search: { modal: "cdn-domains" },
									});
								}}
							>
								<Icon icon={faLink} />
								Mange custom domains
							</CommandItem>
						</CommandGroup>
					) : null}
					{config.matchmaker ? (
						<CommandGroup heading="Matchmaker">
							<CommandItem
								keywords={["matchmaker", "lobbies"]}
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/lobbies",
										params: {
											projectNameId,
											environmentNameId,
										},
									});
								}}
							>
								<Icon icon={faJoystick} />
								Lobbies
							</CommandItem>
							<CommandItem
								keywords={["matchmaker", "logs"]}
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/lobbies/logs",
										params: {
											projectNameId,
											environmentNameId,
										},
									});
								}}
							>
								<Icon icon={faScroll} />
								Logs
							</CommandItem>
							<CommandItem
								keywords={["matchmaker", "settings"]}
								onSelect={() => {
									navigate({
										to: "/projects/$projectNameId/environments/$environmentNameId/lobbies/settings",
										params: {
											projectNameId,
											environmentNameId,
										},
									});
								}}
							>
								<Icon icon={faGear} />
								Settings
							</CommandItem>
						</CommandGroup>
					) : null}
				</>
			) : null}
			<CommandGroup heading="Tokens">
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/environments/$environmentNameId/tokens",
							params: { projectNameId, environmentNameId },
							search: { modal: "public-token" },
						});
					}}
				>
					<Icon icon={faKey} />
					Generate a public token
				</CommandItem>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/environments/$environmentNameId/tokens",
							params: { projectNameId, environmentNameId },
							search: { modal: "service-token" },
						});
					}}
				>
					<Icon icon={faKey} />
					Generate a service token
				</CommandItem>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/environments/$environmentNameId/tokens",
							params: { projectNameId, environmentNameId },
						});
					}}
				>
					<Icon icon={faKey} />
					Generate a development token
				</CommandItem>
			</CommandGroup>
		</>
	);
}
