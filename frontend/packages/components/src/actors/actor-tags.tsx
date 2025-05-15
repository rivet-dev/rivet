import {
	DiscreteCopyButton,
	Slot,
	Slottable,
	WithTooltip,
	cn,
} from "@rivet-gg/components";
import { Icon, faTag } from "@rivet-gg/icons";
import { type ReactNode, forwardRef } from "react";

const BUILT_IN_TAGS = {
	actors: ["framework", "framework-version"],
	builds: ["current"],
};

export const ACTOR_FRAMEWORK_TAG_VALUE = "actor-core";

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
	hoverable?: boolean;
}

export function ActorTags({
	tags = {},
	excludeBuiltIn = undefined,
	truncate = true,
	className,
	hoverable = true,
	copy = true,
}: ActorTagsProps) {
	return (
		<div
			className={cn(
				"flex flex-wrap gap-4 gap-y-2 empty:hidden text-muted-foreground text-xs font-mono",
				truncate && "gap-1",
				className,
			)}
		>
			{tags && typeof tags === "object"
				? Object.entries(tags)
						.filter(([key]) =>
							excludeBuiltIn
								? !BUILT_IN_TAGS[excludeBuiltIn].includes(key)
								: true,
						)
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
						})
				: null}
		</div>
	);
}
