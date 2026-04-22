import { faActorsBorderless, Icon, type IconProp } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import {
	Fragment,
	type LazyExoticComponent,
	lazy,
	type ReactNode,
	Suspense,
} from "react";
import { Button, cn, Skeleton } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { features } from "@/lib/features";
import { RECORDS_PER_PAGE } from "./data-providers/default-data-provider";

const emojiRegex =
	/[\u{1F300}-\u{1F9FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]|[\u{1F600}-\u{1F64F}]|[\u{1F680}-\u{1F6FF}]|[\u{1F1E0}-\u{1F1FF}]/u;

function isEmoji(str: string): boolean {
	return emojiRegex.test(str);
}

function capitalize(str: string): string {
	return str.charAt(0).toUpperCase() + str.slice(1);
}

function toPascalCase(str: string): string {
	return str
		.split("-")
		.map((part) => capitalize(part))
		.join("");
}

const iconModules = import.meta.glob<Record<string, IconProp>>(
	"../../packages/icons/dist/icons/*.js",
);

const lazyIconCache = new Map<
	string,
	LazyExoticComponent<(props: { className?: string }) => ReactNode>
>();

function getLazyIcon(
	iconName: string,
): LazyExoticComponent<(props: { className?: string }) => ReactNode> {
	let component = lazyIconCache.get(iconName);
	if (!component) {
		const loader =
			iconModules[`../../packages/icons/dist/icons/${iconName}.js`];
		component = lazy(() =>
			(loader ? loader() : Promise.reject())
				.then((mod) => ({
					default: ({ className }: { className?: string }) => (
						<Icon
							icon={mod[iconName] ?? faActorsBorderless}
							className={className}
						/>
					),
				}))
				.catch(() => ({
					default: ({ className }: { className?: string }) => (
						<Icon icon={faActorsBorderless} className={className} />
					),
				})),
		);
		lazyIconCache.set(iconName, component);
	}
	return component;
}

function ActorIcon({ iconValue }: { iconValue: string | null }) {
	const className =
		"opacity-80 group-hover:opacity-100 group-data-active:opacity-100";

	if (iconValue && isEmoji(iconValue)) {
		return <span className={`${className} text-sm`}>{iconValue}</span>;
	}

	const iconName = iconValue ? `fa${toPascalCase(iconValue)}` : null;

	if (!iconName) {
		return <Icon icon={faActorsBorderless} className={className} />;
	}

	const LazyIcon = getLazyIcon(iconName);
	return <LazyIcon className={className} />;
}

export function ActorBuildsList() {
	const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(useEngineCompatDataProvider().buildsQueryOptions());

	const navigate = useNavigate();

	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{data?.length === 0 ? (
					<p className="text-xs text-muted-foreground ms-1 px-1">
						Connect RivetKit to see instances.
					</p>
				) : null}
				{data?.map((build) => {
					const actorMeta = build.name.metadata as
						| Record<string, unknown>
						| undefined;
					const iconValue =
						typeof actorMeta?.icon === "string"
							? actorMeta.icon
							: null;
					const displayName =
						typeof actorMeta?.name === "string"
							? actorMeta.name
							: build.id;

					return (
						<Button
							key={build.id}
							className={cn(
								"text-muted-foreground justify-start font-medium px-1",
								"data-active:text-foreground data-active:bg-accent",
							)}
							startIcon={
								<Suspense
									fallback={
										<Icon
											icon={faActorsBorderless}
											className="opacity-80 animate-pulse"
										/>
									}
								>
									<ActorIcon iconValue={iconValue} />
								</Suspense>
							}
							variant={"ghost"}
							size="sm"
							onClick={() => {
								// eslint-disable-next-line @typescript-eslint/no-explicit-any
								return (navigate as any)({
									to: features.multitenancy
										? "/orgs/$organization/projects/$project/ns/$namespace"
										: "/ns/$namespace",
									search: (old: Record<string, unknown>) => ({
										...old,
										actorId: undefined,
										n: [build.id],
									}),
								});
							}}
							asChild
						>
							<Link
								to="."
								search={(old) => ({
									...old,
									actorId: undefined,
									n: [build.id],
								})}
							>
								<span className="text-ellipsis overflow-hidden whitespace-nowrap">
									{displayName}
								</span>
							</Link>
						</Button>
					);
				})}
				{isFetchingNextPage || isLoading
					? Array(RECORDS_PER_PAGE)
							.fill(null)
							.map((_, i) => (
								// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
								<Fragment key={i}>
									<Skeleton className="w-full h-6 my-1" />
								</Fragment>
							))
					: null}
			</div>
			{hasNextPage && !isFetchingNextPage ? (
				<VisibilitySensor onChange={fetchNextPage} />
			) : null}
		</div>
	);
}

export function ActorBuildsListSkeleton() {
	return (
		<div className="h-full">
			<div className="flex flex-col gap-[1px]">
				{Array(RECORDS_PER_PAGE)
					.fill(null)
					.map((_, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
						<Fragment key={i}>
							<Skeleton className="w-full h-6 my-1" />
						</Fragment>
					))}
			</div>
		</div>
	);
}
