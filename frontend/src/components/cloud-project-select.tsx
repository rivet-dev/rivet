import { faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { type ComponentProps, useCallback } from "react";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
	Skeleton,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";

interface CloudProjectSelectProps extends ComponentProps<typeof Select> {
	organization: string;
	showCreateProject?: boolean;
	onCreateClick?: () => void;
}

export function CloudProjectSelect({
	showCreateProject,
	onCreateClick,
	onValueChange,
	organization,
	...props
}: CloudProjectSelectProps) {
	const dataProvider = useCloudDataProvider();
	const { data, hasNextPage, isLoading, isFetchingNextPage, fetchNextPage } =
		useInfiniteQuery(
			dataProvider.projectsQueryOptions({
				organization,
			}),
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
			<SelectTrigger>
				<SelectValue placeholder="Select project..." />
			</SelectTrigger>
			<SelectContent>
				{showCreateProject ? (
					<>
						<SelectItem value="create">
							<Icon className="mr-2 size-4" icon={faPlus} />
							Create project
						</SelectItem>
						<SelectSeparator />
					</>
				) : null}
				{isLoading ? (
					<>
						<SelectItem disabled value="loading-0">
							<Skeleton className="h-4 w-32" />
						</SelectItem>
						<SelectItem disabled value="loading-1">
							<Skeleton className="h-4 w-32" />
						</SelectItem>
						<SelectItem disabled value="loading-2">
							<Skeleton className="h-4 w-32" />
						</SelectItem>
					</>
				) : null}
				{data?.map((project) => (
					<SelectItem key={project.id} value={project.name}>
						{project.displayName || project.name}
					</SelectItem>
				))}
				{isFetchingNextPage ? (
					<SelectItem disabled value="loading-more">
						<Skeleton className="h-4 w-32" />
					</SelectItem>
				) : null}
				{hasNextPage ? (
					<VisibilitySensor onChange={fetchNextPage} />
				) : null}
			</SelectContent>
		</Select>
	);
}
