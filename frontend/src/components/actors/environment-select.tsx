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
import { projectEnvironmentsQueryOptions } from "@/domains/project/queries";

interface EnvironmentSelectProps extends ComponentProps<typeof Select> {
	projectId: string;
	showCreateEnvironment?: boolean;
	onCreateClick?: () => void;
	variant?: ComponentProps<typeof SelectTrigger>["variant"];
}

export function EnvironmentSelect({
	showCreateEnvironment,
	onCreateClick,
	onValueChange,
	projectId,
	variant,
	...props
}: EnvironmentSelectProps) {
	const { data } = useSuspenseQuery(
		projectEnvironmentsQueryOptions(projectId),
	);

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
			<SelectTrigger variant={variant}>
				<SelectValue placeholder="Select environment..." />
			</SelectTrigger>
			<SelectContent>
				{showCreateEnvironment ? (
					<>
						<SelectItem value="create">
							<Flex gap="2" items="center">
								<Icon className="size-4" icon={faCirclePlus} />
								Create new environment
							</Flex>
						</SelectItem>
						<SelectSeparator />
					</>
				) : null}
				{data.map((environment) => (
					<SelectItem
						key={environment.namespaceId}
						value={environment.namespaceId}
					>
						{environment.displayName}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
