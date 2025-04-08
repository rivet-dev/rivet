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
}

export function ActorTags({
	tags = {},
	excludeBuiltIn = undefined,
	truncate = true,
	className,
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
						.map(([key, value]) =>
							truncate ? (
								<WithTooltip
									key={key}
									content={`${key}=${value}`}
									trigger={
										<DiscreteCopyButton
											size="xs"
											value={`${key}=${value}`}
										>
											<ActorTag className="flex-shrink-0 truncate max-w-52 cursor-pointer">
												<button type="button">
													{key}={value}
												</button>
											</ActorTag>
										</DiscreteCopyButton>
									}
								/>
							) : (
								<ActorTag key={key} className="flex-shrink-0">
									<span>
										{key}={value}
									</span>
								</ActorTag>
							),
						)
				: null}
		</div>
	);
}
