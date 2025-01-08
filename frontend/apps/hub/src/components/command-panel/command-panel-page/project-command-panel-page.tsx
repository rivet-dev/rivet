import {
	projectByIdQueryOptions,
	projectQueryOptions,
} from "@/domains/project/queries";
import { GuardEnterprise } from "@/lib/guards";
import { CommandGroup, CommandItem } from "@rivet-gg/components";
import { Icon, faCircleDollar, faCog, faHome, faKey } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useCommandPanelNavigation } from "../command-panel-navigation-provider";
import { EnvironmentsCommandPanelItems } from "../environments-command-panel-items";

interface ProjectCommandPanelPage {
	projectNameId: string;
}

export function ProjectCommandPanelPage({
	projectNameId,
}: ProjectCommandPanelPage) {
	const { data } = useSuspenseQuery(projectByIdQueryOptions(projectNameId));

	const { data: project } = useSuspenseQuery(
		projectQueryOptions(data.gameId),
	);

	const { navigate } = useCommandPanelNavigation();

	return (
		<>
			<CommandGroup heading={data.displayName}>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId",
							params: { projectNameId },
						});
					}}
				>
					<Icon icon={faHome} />
					Overview
				</CommandItem>
				<GuardEnterprise>
					<CommandItem
						onSelect={() => {
							navigate({
								to: "/projects/$projectNameId/billing",
								params: { projectNameId },
							});
						}}
					>
						<Icon icon={faCircleDollar} />
						Billing
					</CommandItem>
				</GuardEnterprise>
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId/settings",
							params: { projectNameId },
						});
					}}
				>
					<Icon icon={faCog} />
					Settings
				</CommandItem>
			</CommandGroup>
			<CommandGroup heading="Environments">
				<EnvironmentsCommandPanelItems
					projectNameId={projectNameId}
					namespaces={project.namespaces}
				/>
			</CommandGroup>
			<CommandGroup heading="Tokens">
				<CommandItem
					onSelect={() => {
						navigate({
							to: "/projects/$projectNameId",
							params: { projectNameId },
							search: { modal: "cloud-token" },
						});
					}}
				>
					<Icon icon={faKey} />
					Generate a cloud token
				</CommandItem>
			</CommandGroup>
		</>
	);
}
