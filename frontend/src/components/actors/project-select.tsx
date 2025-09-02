import { faCirclePlus, Icon } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { type ComponentProps, Fragment, useCallback } from "react";
import {
	Flex,
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { GroupAvatar } from "@/domains/group/components/group-avatar";
import { projectsByGroupQueryOptions } from "@/domains/project/queries";

interface ProjectSelectProps extends ComponentProps<typeof Select> {
	showCreateProject?: boolean;
	onCreateClick?: () => void;
}

export function ProjectSelect({
	showCreateProject,
	onCreateClick,
	onValueChange,
	...props
}: ProjectSelectProps) {
	const { data } = useSuspenseQuery(projectsByGroupQueryOptions());

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
		<Select {...props} onValueChange={handleValueChange}>
			<SelectTrigger>
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
				{data.map((group, index, groupList) => (
					<Fragment key={group.groupId}>
						<SelectGroup>
							<SelectLabel>
								<Flex gap="2" items="center">
									<GroupAvatar
										className="size-5"
										displayName={group.displayName}
										avatarUrl={group.avatarUrl}
									/>
									{group.displayName}
								</Flex>
							</SelectLabel>
							{group.projects.map((project) => (
								<SelectItem
									className="pl-8"
									key={project.gameId}
									value={project.gameId}
								>
									{project.displayName}
								</SelectItem>
							))}
						</SelectGroup>
						{groupList.length - 1 !== index ? (
							<SelectSeparator />
						) : null}
					</Fragment>
				))}
			</SelectContent>
		</Select>
	);
}
