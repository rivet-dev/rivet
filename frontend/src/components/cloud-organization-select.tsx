import { faPlus, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
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
	const { data = [], isLoading } = useQuery(
		useCloudDataProvider().organizationsQueryOptions(),
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
				{data.map((org) => (
					<SelectItem key={org.id} value={org.slug}>
						<span className="inline-flex items-center gap-2">
							<Avatar className="size-5">
								<AvatarImage src={org.logo ?? undefined} />
								<AvatarFallback>
									{org.name?.[0]?.toUpperCase()}
								</AvatarFallback>
							</Avatar>
							<span className="truncate">{org.name}</span>
						</span>
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
