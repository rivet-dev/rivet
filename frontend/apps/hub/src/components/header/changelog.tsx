import { changelogQueryOptions } from "@/domains/user/queries";
import type { ChangelogItem } from "@/domains/user/queries/type";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Badge,
	Picture,
	PictureFallback,
	PictureImage,
	Ping,
	Skeleton,
	WithTooltip,
	cn,
} from "@rivet-gg/components";
import { Icon, faExternalLinkAlt, faSparkle } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useLocalStorage } from "usehooks-ts";
import { NavItem } from "./nav-item";

interface ChangelogEntryProps extends ChangelogItem {}

export function ChangelogEntry({
	published,
	images,
	title,
	description,
	slug,
	authors,
}: ChangelogEntryProps) {
	return (
		<div className="py-2">
			<div className="flex my-2 justify-between items-center">
				<div className="flex items-center gap-2">
					<div className="bg-white text-background size-8 rounded-full flex items-center justify-center">
						<Icon icon={faSparkle} className="m-0" />
					</div>
					<h4 className="font-bold text-lg">New Update</h4>
				</div>
				<Badge variant="outline">
					{new Date(published).toLocaleDateString()}
				</Badge>
			</div>

			<div>
				<Picture className="rounded-md border my-4 h-[200px] w-full block overflow-hidden">
					<PictureFallback>
						<Skeleton className="size-full" />
					</PictureFallback>
					<PictureImage
						className="size-full object-cover animate-in fade-in-0 duration-300 fill-mode-forwards"
						src={`https://rivet.gg/${images[0].url}`}
						alt={"Changelog entry"}
					/>
				</Picture>

				<p className="font-semibold text-sm">{title}</p>

				<p className="text-xs mt-1 text-muted-foreground">
					{description}
					<a
						className="text-right text-xs flex gap-1.5 text-foreground items-center"
						href={`https://rivet.gg/changelog/${slug}`}
						target="_blank"
						rel="noreferrer"
					>
						Read more <Icon icon={faExternalLinkAlt} />
					</a>
				</p>
				<div className="flex items-end justify-end mt-2">
					<div className="flex gap-2 items-center">
						<a
							className="flex gap-1.5 items-center flex-row-reverse text-right"
							href={authors[0].url}
						>
							<Avatar className="size-8">
								<AvatarFallback>
									{authors[0].name[0]}
								</AvatarFallback>
								<AvatarImage
									src={`https://rivet.gg/${authors[0].avatar.url}`}
									alt={authors[0].name}
								/>
							</Avatar>
							<div className="ml-2">
								<p className="font-semibold text-sm">
									{authors[0].name}
								</p>
								<p className="text-xs text-muted-foreground">
									{authors[0].role}
								</p>
							</div>
						</a>
					</div>
				</div>
			</div>
		</div>
	);
}
interface ChangelogProps {
	className?: string;
}

export function Changelog({ className }: ChangelogProps) {
	const { data } = useSuspenseQuery(changelogQueryOptions());

	const [lastChangelog, setLast] = useLocalStorage<string | null>(
		"rivet-lastchangelog",
		null,
	);

	const hasNewChangelog = !lastChangelog
		? data.length > 0
		: data.some(
				(entry) => new Date(entry.published) > new Date(lastChangelog),
			);

	const trigger = (
		<NavItem
			asChild
			className={cn(
				"hidden md:inline-block relative py- 2data-open:text-foreground",
				className,
			)}
		>
			<a
				href="https://rivet.gg/changelog"
				target="_blank"
				rel="noreferrer"
			>
				{hasNewChangelog ? <Ping /> : null}
				Changelog
			</a>
		</NavItem>
	);

	if (data.length === 0) {
		return trigger;
	}

	return (
		<WithTooltip
			delayDuration={0}
			onOpenChange={(isOpen) => {
				if (isOpen) {
					setLast(data[0].published);
				}
			}}
			trigger={trigger}
			content={<ChangelogEntry {...data[0]} />}
		/>
	);
}
