import { faTag, Icon } from "@rivet-gg/icons";
import { forwardRef, type ReactNode, useState } from "react";
import {
	Button,
	cn,
	DiscreteCopyButton,
	Slot,
	Slottable,
	WithTooltip,
} from "@/components";

const BUILT_IN_TAGS = {
	actors: ["framework", "framework-version"],
	builds: ["current"],
};

export const ACTOR_FRAMEWORK_TAG_VALUE = "rivetkit";

export const ActorTag = forwardRef<
	HTMLSpanElement,
	{ children: ReactNode; className?: string }
>(({ children, className, ...props }, ref) => (
	<Slot ref={ref} className={className} {...props}>
		<Icon className="mr-1" icon={faTag} />
		<Slottable>{children}</Slottable>
	</Slot>
));

interface ActorTagsProps {
	tags?: unknown;
	excludeBuiltIn?: keyof typeof BUILT_IN_TAGS;
	className?: string;
	truncate?: boolean;
	copy?: boolean;
	max?: number;
	hoverable?: boolean;
}

export function ActorTags({
	tags = {},
	excludeBuiltIn = undefined,
	truncate = true,
	className,
	hoverable = true,
	max = Number.POSITIVE_INFINITY,
	copy = true,
}: ActorTagsProps) {
	const withoutBuiltIn = Object.entries(tags ?? {}).filter(([key]) =>
		excludeBuiltIn ? !BUILT_IN_TAGS[excludeBuiltIn].includes(key) : true,
	);

	const [isTruncatedList, setTruncatedList] = useState(
		withoutBuiltIn.length > max,
	);

	const truncated = withoutBuiltIn.filter((_, index) =>
		isTruncatedList ? index < max : true,
	);

	const truncatedCount = withoutBuiltIn.length - truncated.length;

	return (
		<div
			className={cn(
				"flex flex-wrap gap-4 gap-y-2 empty:hidden text-muted-foreground text-xs font-mono",
				truncate && "gap-1",
				className,
			)}
		>
			{truncated.length > 0 ? (
				<>
					{truncated
						.sort(([a], [b]) => a.localeCompare(b))
						.map(([key, value]) => {
							let trigger = truncate ? (
								<ActorTag
									key={key}
									className="break-all truncate max-w-52 cursor-pointer inline"
								>
									<span>
										{key}={value}
									</span>
								</ActorTag>
							) : (
								<ActorTag key={key}>
									<span className="inline break-all max-w-full whitespace-normal">
										{key}={value}
									</span>
								</ActorTag>
							);

							trigger = copy ? (
								<DiscreteCopyButton
									key={key}
									size="xs"
									className={cn(
										"h-auto py-0.5 text-left max-w-full min-w-0 break-all",
										truncate && "flex max-w-52",
									)}
									value={`${key}=${value}`}
								>
									<Slot className="mr-1 inline">
										{trigger}
									</Slot>
								</DiscreteCopyButton>
							) : (
								trigger
							);

							return truncate && hoverable && !copy ? (
								<WithTooltip
									key={key}
									content={`${key}=${value}`}
									trigger={trigger}
								/>
							) : (
								trigger
							);
						})}

					{truncatedCount > 0 ? (
						<Button
							variant="ghost"
							size="xs"
							className="h-auto py-0.5 text-left max-w-full min-w-0 break-all"
							onClick={() => {
								setTruncatedList(false);
							}}
						>
							<ActorTag className="cursor-pointer">
								<span className="inline">
									+{truncatedCount} more
								</span>
							</ActorTag>
						</Button>
					) : null}
				</>
			) : null}
		</div>
	);
}
