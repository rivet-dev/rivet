import { faCirclePlus, Icon } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type ComponentProps, useCallback } from "react";
import {
	Flex,
	Select,
	SelectContent,
	SelectItem,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { groupProjectsQueryOptions } from "@/domains/project/queries";

interface GroupProjectSelectProps extends ComponentProps<typeof Select> {
	groupId: string;
	showCreateProject?: boolean;
	onCreateClick?: () => void;
	variant?: ComponentProps<typeof SelectTrigger>["variant"];
}

export function GroupProjectSelect({
	groupId,
	showCreateProject,
	onCreateClick,
	onValueChange,
	variant,
	...props
}: GroupProjectSelectProps) {
	const { data } = useSuspenseQuery(groupProjectsQueryOptions(groupId));

	const handleValueChange = useCallback(
		(value: string) => {
			if (value === "create") {
				onCreateClick?.();
				return;
			}
			onValueChange?.(value);
		},
		[onCreateClick, onValueChange],
	);

	return (
		<Select onValueChange={handleValueChange} {...props}>
			<SelectTrigger variant={variant}>
				<SelectValue placeholder="Select project..." />
			</SelectTrigger>
			<SelectContent>
				{showCreateProject ? (
					<>
						<SelectItem value="create">
							<Flex gap="2" items="center">
								<Icon className="size-4" icon={faCirclePlus} />
								Create new project
							</Flex>
						</SelectItem>
						<SelectSeparator />
					</>
				) : null}
				{data.projects.map((project) => (
					<SelectItem key={project.gameId} value={project.gameId}>
						{project.displayName}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
