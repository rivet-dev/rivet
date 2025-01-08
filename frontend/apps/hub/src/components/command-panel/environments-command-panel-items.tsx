import type { Environment } from "@/domains/project/queries";
import { Badge, CommandItem } from "@rivet-gg/components";
import { useCommandPanelNavigation } from "./command-panel-navigation-provider";

interface EnvironmentsCommandPanelItemsProps {
	namespaces: Environment[];
	projectNameId: string;
}

export function EnvironmentsCommandPanelItems({
	namespaces,
	projectNameId,
}: EnvironmentsCommandPanelItemsProps) {
	const { changePage } = useCommandPanelNavigation();
	return (
		<>
			{namespaces.map((environment) => (
				<CommandItem
					key={environment.namespaceId}
					onSelect={() => {
						changePage({
							key: "environment",
							params: {
								projectNameId,
								environmentNameId: environment.nameId,
							},
						});
					}}
				>
					{environment.displayName}{" "}
					{environment.version ? (
						<Badge className="ml-2">
							{environment.version?.displayName}
						</Badge>
					) : null}
				</CommandItem>
			))}
		</>
	);
}
