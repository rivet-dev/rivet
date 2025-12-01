import { faChevronRight, faCode, Icon } from "@rivet-gg/icons";
import { templates } from "@rivetkit/example-registry";
import { createLink, Link, type LinkOptions } from "@tanstack/react-router";
import { motion } from "framer-motion";
import React, { type ComponentProps, useState } from "react";
import { Picture, PictureFallback, PictureImage } from "@/components";
import { cn } from "../components/lib/utils";
import { Button } from "../components/ui/button";
import { H1 } from "../components/ui/typography";

export function GettingStarted({
	getTemplateLink,
	connextExistingProjectLink,
	showFooter = true,
}: {
	getTemplateLink?: (template: string) => LinkOptions;
	connextExistingProjectLink?: LinkOptions;
	showFooter?: boolean;
}) {
	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg overflow-auto @container">
			<div className="max-w-5xl mx-auto">
				<div className="mt-2 flex justify-between items-center px-10 py-4">
					<H1>Get Started</H1>
				</div>
			</div>

			<hr className="mb-6 mt-2" />

			<div className="px-4">
				<div className="px-6 max-w-5xl mx-auto my-8">
					<Templates getTemplateLink={getTemplateLink} />
				</div>

				{showFooter ? (
					<div className="w-full mb-6">
						<div className=" w-full bg-card"></div>
						<div className="px-6 w-full flex justify-stretch gap-16 mx-auto max-w-5xl">
							<Button
								variant="secondary"
								className="w-full bg-card py-4 h-auto"
								asChild
							>
								<Link
									to="."
									search={{
										modal: "connect-existing-project",
									}}
									{...(connextExistingProjectLink || {})}
								>
									Connect Existing Project
								</Link>
							</Button>
							<Button
								variant="secondary"
								className="w-full bg-card py-4 h-auto"
								asChild
							>
								<a
									href="https://www.rivet.dev/docs"
									rel="noopener noreferrer"
									target="_blank"
								>
									Create New Project
								</a>
							</Button>
						</div>
					</div>
				) : null}
			</div>
		</div>
	);
}

function Templates({
	getTemplateLink,
}: {
	getTemplateLink?: (template: string) => LinkOptions;
}) {
	const [showAll, setShowAll] = useState(false);
	return (
		<>
			<motion.div
				className="grid grid-cols-2 gap-4 gap-y-10 my-4 @6xl:grid-cols-2"
				variants={{ hidden: { opacity: 0.8 }, show: { opacity: 1 } }}
				initial="hidden"
				animate="show"
			>
				{templates
					.toSorted(
						(a, b) =>
							(a.priority || Number.MAX_SAFE_INTEGER) -
							(b.priority || Number.MAX_SAFE_INTEGER),
					)
					.slice(0, showAll ? templates.length : 4)
					.map((template) => (
						<TemplateCard
							getLink={getTemplateLink}
							key={template.name}
							slug={template.name}
							title={template.displayName}
							description={template.description}
						/>
					))}
			</motion.div>
			<div className="flex justify-center col-span-full mt-8">
				{!showAll && templates.length > 4 && (
					<Button
						variant="ghost"
						onClick={() => setShowAll(true)}
						endIcon={<Icon icon={faChevronRight} />}
					>
						See All
					</Button>
				)}
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
				"h-auto flex-col items-start text-wrap pt-4 px-4 overflow-hidden group",
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
	return <motion.a ref={ref} {...props} />;
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
			<div className="absolute rounded-b-xl bottom-0 inset-x-0 right-0 h-full bg-card-fade group-hover:h-3/4 transition-all" />
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
					className="size-full object-cover aspect-video rounded-b-xl"
					src={`/examples/${slug}/image.png`}
					alt={title}
				/>
			</Picture>
		</div>
	);
}
