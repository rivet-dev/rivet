import { faSparkle, Icon } from "@rivet-gg/icons";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useLocalStorage } from "usehooks-ts";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Badge,
	cn,
	Picture,
	PictureFallback,
	PictureImage,
	Skeleton,
	Slot,
	WithTooltip,
} from "@/components";
import { changelogQueryOptions } from "@/queries/global";
import type { ChangelogItem } from "@/queries/types";

interface ChangelogEntryProps extends ChangelogItem {
	isNew?: boolean;
}

export function ChangelogEntry({
	published,
	images,
	title,
	description,
	slug,
	authors,
	isNew,
}: ChangelogEntryProps) {
	return (
		<div className="py-2">
			<div className="flex my-2 justify-between items-center">
				<div className="flex items-center gap-2">
					<div className="bg-white text-background size-8 rounded-full flex items-center justify-center">
						<Icon icon={faSparkle} className="m-0" />
					</div>
					<h4 className=" text-lg text-foreground">
						{isNew ? (
							<span>New Update</span>
						) : (
							<span>Latest Update</span>
						)}
					</h4>
				</div>
				<Badge variant="outline">
					{new Date(published).toLocaleDateString()}
				</Badge>
			</div>

			<a
				href={`https://www.rivet.dev/changelog/${slug}`}
				target="_blank"
				rel="noreferrer"
				className="block"
			>
				<Picture className="rounded-md border my-4 h-[200px] w-full block overflow-hidden aspect-video">
					<PictureFallback>
						<Skeleton className="size-full" />
					</PictureFallback>
					<PictureImage
						className="size-full object-cover animate-in fade-in-0 duration-300 fill-mode-forwards"
						src={`https://www.rivet.dev/${images[0].url}`}
						width={images[0].width}
						height={images[0].height}
						alt={"Changelog entry"}
					/>
				</Picture>

				<p className="font-semibold text-sm">{title}</p>

				<p className="text-xs mt-1 text-muted-foreground">
					{description}{" "}
					<span className="text-right text-xs inline gap-1.5 text-foreground items-center">
						Read more...
					</span>
				</p>
			</a>
			<div className="flex items-end justify-end mt-2">
				<div className="flex gap-2 items-center">
					<a
						className="flex gap-1.5 items-center flex-row-reverse text-right"
						href={authors[0].socials.twitter}
					>
						<Avatar className="size-8">
							<AvatarFallback>
								{authors[0].name[0]}
							</AvatarFallback>
							<AvatarImage
								src={`https://www.rivet.dev/${authors[0].avatar.url}`}
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
	);
}
interface ChangelogProps {
	className?: string;
	children?: React.ReactNode;
}

export function Changelog({ className, children, ...props }: ChangelogProps) {
	const { data, isLoading } = useQuery(changelogQueryOptions());

	const [lastChangelog, setLast] = useLocalStorage<string | null>(
		"rivet-lastchangelog",
		null,
	);

	if (isLoading || !data) {
		return (
			<Slot
				{...props}
				className={cn(
					"relative [&_[data-changelog-ping]]:hidden",
					className,
				)}
			>
				{children}
			</Slot>
		);
	}

	const hasNewChangelog = !lastChangelog
		? data.length > 0
		: data.some(
				(entry) => new Date(entry.published) > new Date(lastChangelog),
			);

	return (
		<WithTooltip
			delayDuration={0}
			contentProps={{ collisionPadding: 8 }}
			onOpenChange={(isOpen) => {
				if (isOpen) {
					setLast(data[0].published);
				}
			}}
			trigger={
				<Slot
					{...props}
					className={cn(
						"relative",
						!hasNewChangelog && "[&_[data-changelog-ping]]:hidden",
						className,
					)}
				>
					{children}
				</Slot>
			}
			content={<ChangelogEntry {...data[0]} isNew={hasNewChangelog} />}
		/>
	);
}
