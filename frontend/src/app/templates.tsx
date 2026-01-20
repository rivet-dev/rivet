import { faChevronRight, faCode, Icon } from "@rivet-gg/icons";
import { templates } from "@rivetkit/example-registry";
import {
	createLink,
	Link,
	type LinkOptions,
	useSearch,
} from "@tanstack/react-router";
import { motion } from "framer-motion";
import React, { type ComponentProps } from "react";
import {
	Button,
	cn,
	Picture,
	PictureFallback,
	PictureImage,
} from "@/components";

export function Templates({
	getTemplateLink,
	startFromScratchLink,
}: {
	getTemplateLink?: (template: string) => LinkOptions;
	startFromScratchLink?: LinkOptions;
}) {
	const showAll = useSearch({ strict: false, select: (s) => s?.showAll });
	return (
		<>
			<div className="grid grid-cols-3 gap-4 gap-y-10 my-4">
				{templates
					.toSorted(
						(a, b) =>
							(a.priority || Number.MAX_SAFE_INTEGER) -
							(b.priority || Number.MAX_SAFE_INTEGER),
					)
					.slice(0, showAll ? templates.length : 6)
					.map((template) => (
						<TemplateCard
							getLink={getTemplateLink}
							key={template.name}
							slug={template.name}
							title={template.displayName}
							description={template.description}
						/>
					))}
			</div>

			<div className="flex flex-col items-center justify-center gap-6 mb-8">
				<div className="flex justify-center col-span-full">
					{!showAll && templates.length > 4 && (
						<Button
							variant="ghost"
							asChild
							endIcon={<Icon icon={faChevronRight} />}
						>
							<Link to="." search={{ showAll: true }}>
								See All
							</Link>
						</Button>
					)}
				</div>
				<div className="flex gap-4">
					<Button variant="outline" asChild>
						<Link {...startFromScratchLink}>
							Start From Scratch
						</Link>
					</Button>
				</div>
			</div>
		</>
	);
}

function TemplateCard({
	title,
	slug,
	className,
	description,
	getLink = (slug) => ({
		to: ".",
		search: { modal: "start-with-template", name: slug },
	}),
}: {
	title: string;
	slug: string;
	description?: string;
	className?: string;
	getLink?: (template: string) => LinkOptions;
}) {
	return (
		<Button
			size="lg"
			variant="outline"
			className={cn(
				"h-auto bg-card flex-col items-start text-wrap pt-4 px-4 overflow-hidden group",
				className,
			)}
			asChild
		>
			<MotionLink
				{...(getLink?.(slug) || {})}
				variants={{
					hidden: { opacity: 0, y: 10 },
					show: { opacity: 1, y: 0 },
				}}
			>
				<div className="flex w-full">
					<div className="flex-1 p-2 gap-1.5 flex flex-col w-full">
						<p className="text-base line-clamp-1">{title}</p>
						{description && (
							<p className="text-muted-foreground text-xs line-clamp-2 min-h-8">
								{description}
							</p>
						)}
					</div>
					<Icon
						icon={faChevronRight}
						className="ml-auto mt-3 pl-4 text-muted-foreground group-hover:opacity-100 opacity-0 transition-opacity"
					/>
				</div>

				<ExamplePreview slug={slug} title={title} />
			</MotionLink>
		</Button>
	);
}

const MotionLinkComponent = React.forwardRef<
	HTMLAnchorElement,
	ComponentProps<typeof motion.a>
>((props, ref) => {
	return (
		<motion.a
			layout
			variants={{
				hidden: { opacity: 0, y: 10 },
				show: { opacity: 1, y: 0 },
			}}
			animate="show"
			initial="hidden"
			ref={ref}
			{...props}
		/>
	);
});

const MotionLink = createLink(MotionLinkComponent);

export function ExamplePreview({
	slug,
	title,
	className,
}: {
	slug: string;
	title: string;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"rounded-xl border relative -bottom-4 w-full group-hover:-bottom-3 transition-all overflow-hidden",
				className,
			)}
		>
			<div className="absolute bottom-0 inset-x-0 right-0 h-full bg-card-fade group-hover:h-3/4 transition-all" />
			<div className="flex items-center gap-2 border-b bg-muted px-3 py-2">
				<div className="flex gap-1.5">
					<div className="size-2 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
					<div className="size-2 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
					<div className="size-2 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
				</div>
			</div>

			<Picture className="border-b min-h-40 w-full rounded-b-xl ">
				<PictureFallback>
					<div className=" aspect-video flex">
						<Icon
							icon={faCode}
							className="m-auto text-muted-foreground text-5xl"
						/>
					</div>
				</PictureFallback>
				<PictureImage
					className="size-full object-cover aspect-video"
					src={`/examples/${slug}/image.png`}
					alt={title}
				/>
			</Picture>
		</div>
	);
}
