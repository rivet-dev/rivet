import { faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { type ComponentProps, useCallback } from "react";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Select,
	SelectContent,
	SelectItem,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
	Skeleton,
} from "@/components";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { useCloudDataProvider } from "./actors";

interface CloudOrganizationSelectProps extends ComponentProps<typeof Select> {
	showCreateOrganization?: boolean;
	onCreateClick?: () => void;
}

export function CloudOrganizationSelect({
	showCreateOrganization,
	onCreateClick,
	onValueChange,
	...props
}: CloudOrganizationSelectProps) {
	const {
		data = [],
		isLoading,
		hasNextPage,
		fetchNextPage,
	} = useInfiniteQuery(useCloudDataProvider().organizationsQueryOptions());

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
				<SelectValue placeholder="Select organization..." />
			</SelectTrigger>
			<SelectContent>
				{showCreateOrganization ? (
					<>
						<SelectItem value="create">
							<Icon className="mr-2 size-4" icon={faPlus} />
							Create organization
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
				{data.map((membership) => (
					<SelectItem
						key={membership.id}
						value={membership.organization.id}
					>
						<span className="inline-flex items-center gap-2">
							<Avatar className="size-5">
								<AvatarImage
									src={membership.organization.imageUrl}
								/>
								<AvatarFallback>
									{membership.organization.name?.[0]?.toUpperCase()}
								</AvatarFallback>
							</Avatar>
							<span className="truncate">
								{membership.organization.name}
							</span>
						</span>
					</SelectItem>
				))}
				{hasNextPage ? (
					<VisibilitySensor onChange={fetchNextPage} />
				) : null}
			</SelectContent>
		</Select>
	);
}
