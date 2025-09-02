import { SelectGroup } from "@radix-ui/react-select";
import { faCirclePlus, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { type ComponentProps, useCallback, useEffect, useRef } from "react";
import {
	Flex,
	Select,
	SelectContent,
	SelectItem,
	SelectLabel,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { namespacesQueryOptions } from "@/queries/manager-engine";

interface NamespaceSelectProps extends ComponentProps<typeof Select> {
	showCreate?: boolean;
	onCreateClick?: () => void;
	variant?: ComponentProps<typeof SelectTrigger>["variant"];
	className?: string;
}

export function NamespaceSelect({
	showCreate,
	onCreateClick,
	onValueChange,
	variant,
	className,
	...props
}: NamespaceSelectProps) {
	const { data, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(namespacesQueryOptions());

	const handleValueChange = useCallback(
		(value: string) => {
			if (value === "%%create%%") {
				onCreateClick?.();
				return;
			}
			onValueChange?.(value);
		},
		[onCreateClick, onValueChange],
	);

	return (
		<Select {...props} onValueChange={handleValueChange}>
			<SelectTrigger variant={variant} className={className}>
				<SelectValue placeholder="Select namespace..." />
			</SelectTrigger>
			<SelectContent>
				<SelectGroup>
					<SelectLabel>Namespaces</SelectLabel>
					{data?.map((namespace) => (
						<SelectItem
							key={namespace.namespaceId}
							value={namespace.name}
						>
							{namespace.displayName}
						</SelectItem>
					))}
					{showCreate ? (
						<>
							<SelectSeparator />
							<SelectItem value="%%create%%">
								<Flex gap="2" items="center">
									<Icon
										className="size-4"
										icon={faCirclePlus}
									/>
									Create new namespace
								</Flex>
							</SelectItem>
						</>
					) : null}
					{isFetchingNextPage ? (
						<div className="w-full flex items-center py-1">
							<Icon
								icon={faSpinnerThird}
								className="animate-spin size-4 mx-auto"
							/>
						</div>
					) : null}
					{hasNextPage ? (
						<VisibilitySensor onChange={fetchNextPage} />
					) : null}
				</SelectGroup>
			</SelectContent>
		</Select>
	);
}
