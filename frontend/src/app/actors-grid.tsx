import { faGear, faPlus, Icon } from "@rivet-gg/icons";
import {
	queryOptions,
	useInfiniteQuery,
	useQueries,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { type ReactNode } from "react";
import {
	Button,
	cn,
	H1,
	ScrollArea,
	Skeleton,
	SmallText,
	WithTooltip,
} from "@/components";
import { ActorIcon } from "@/components/lazy-icon";
import { useDataProvider, useCloudNamespaceDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { ImagesTable } from "@/app/images-table";
import { NoProvidersAlert } from "@/components/actors/no-providers-alert";

function GridCard({
	children,
	className,
	asChild,
	onClick,
}: {
	children: ReactNode;
	className?: string;
	asChild?: boolean;
	onClick?: () => void;
}) {
	const Wrapper = asChild ? "div" : "button";
	return (
		<Wrapper
			onClick={onClick}
			className={cn(
				"group relative flex flex-col items-start gap-2 rounded-lg border border-foreground/10 bg-foreground/[0.02] p-4 text-left transition-colors",
				"hover:border-foreground/20 hover:bg-foreground/[0.05]",
				"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
				"min-h-[110px]",
				className,
			)}
			type={asChild ? undefined : "button"}
		>
			{children}
		</Wrapper>
	);
}

function ActorGridCardSkeleton() {
	return (
		<div className="flex min-h-[110px] flex-col items-start gap-2 rounded-lg border border-foreground/10 bg-foreground/[0.02] p-4">
			<Skeleton className="h-9 w-9 rounded-md" />
			<Skeleton className="h-4 w-24" />
			<Skeleton className="h-3 w-16" />
		</div>
	);
}

export function ActorsGrid({
	namespaceLabel,
}: {
	namespaceLabel?: string;
}) {
	const dataProvider = useDataProvider();
	const navigate = useNavigate();
	const { data, isLoading, hasNextPage, fetchNextPage, isFetchingNextPage } =
		useInfiniteQuery(dataProvider.buildsQueryOptions());
	const { data: runnerNamesCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerNamesQueryOptions(),
		select: (data) => data.pages.flatMap((page) => page.names).length,
	});
	const { data: runnerConfigsCount = 0 } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		select: (data) =>
			data.pages.flatMap((page) => Object.keys(page.runnerConfigs))
				.length,
	});
	const hasRunners = runnerNamesCount + runnerConfigsCount > 0;

	const builds = data ?? [];
	const sorted = builds.toSorted((a, b) => a.id.localeCompare(b.id));

	const namespaceName = namespaceLabel ?? "Namespace";

	return (
		<div className="flex flex-1 min-h-0 my-2 mr-2 overflow-hidden rounded-xl border border-foreground/10 bg-card">
			<ScrollArea className="h-full w-full">
				<div className="px-6 py-6 max-w-6xl mx-auto space-y-8">
					<header className="flex items-center justify-between gap-4 pb-6 border-b border-foreground/10">
						<H1 className="text-2xl truncate">{namespaceName}</H1>
						<WithTooltip
							content="Namespace settings"
							trigger={
								<Button
									variant="outline"
									size="icon-sm"
									aria-label="Namespace settings"
									onClick={() => {
										navigate({
											to: ".",
											search: (old) => ({
												...old,
												settings: "settings",
											}),
										});
									}}
								>
									<Icon icon={faGear} />
								</Button>
							}
						/>
					</header>

					<section>
						<header className="flex items-center justify-between gap-4 mb-3">
							<h2 className="text-base font-semibold text-foreground">
								Actors
							</h2>
							{builds.length > 0 ? (
								<Button
									variant="outline"
									size="sm"
									startIcon={<Icon icon={faPlus} />}
									onClick={() => {
										navigate({
											to: ".",
											search: (old) => ({
												...old,
												modal: "create-actor",
											}),
										});
									}}
								>
									Create Actor
								</Button>
							) : null}
						</header>

						{isLoading ? (
							<div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
								{Array.from({ length: 8 }).map((_, i) => (
									// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
									<ActorGridCardSkeleton key={i} />
								))}
							</div>
						) : builds.length === 0 ? (
							!hasRunners ? (
								<NoProvidersAlert variant="connect" />
							) : (
								<div className="flex flex-col items-center gap-3 rounded-md border border-dashed bg-card/50 px-6 py-10 text-center">
									<h3 className="text-base font-semibold text-foreground">
										No actors yet
									</h3>
									<SmallText className="text-muted-foreground max-w-md">
										Deploy code that registers an actor to see
										it here.
									</SmallText>
									<Button
										variant="default"
										size="sm"
										startIcon={<Icon icon={faPlus} />}
										onClick={() => {
											navigate({
												to: ".",
												search: (old) => ({
													...old,
													modal: "create-actor",
												}),
											});
										}}
									>
										Create Actor
									</Button>
								</div>
							)
						) : (
							<>
								<div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
								{sorted.map((build) => {
									const meta = build.name.metadata as
										| Record<string, unknown>
										| undefined;
									const iconValue =
										typeof meta?.icon === "string"
											? meta.icon
											: null;
									const displayName =
										typeof meta?.name === "string"
											? meta.name
											: build.id;

									return (
										<Link
											key={build.id}
											to="."
											search={(old) => ({
												...old,
												actorId: undefined,
												actorKey: undefined,
												n: [build.id],
											})}
											className={cn(
												"group relative flex flex-col items-start gap-2 rounded-lg border border-foreground/10 bg-foreground/[0.02] p-4 text-left transition-colors",
												"hover:border-foreground/20 hover:bg-foreground/[0.05]",
												"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
												"min-h-[110px] cursor-pointer",
											)}
										>
											<div className="flex h-9 w-9 items-center justify-center rounded-md bg-foreground/[0.06] text-foreground/80">
												<ActorIcon
													iconValue={iconValue}
													className="text-lg"
												/>
											</div>
											<div className="font-medium text-sm leading-tight">
												{displayName}
											</div>
											{displayName !== build.id ? (
												<SmallText className="text-muted-foreground text-xs leading-tight">
													{build.id}
												</SmallText>
											) : null}
										</Link>
									);
								})}
								{isFetchingNextPage
									? Array.from({ length: 4 }).map((_, i) => (
											// biome-ignore lint/suspicious/noArrayIndexKey: skeleton loaders are static
											<ActorGridCardSkeleton key={`next-${i}`} />
										))
									: null}
								</div>
								{hasNextPage && !isFetchingNextPage ? (
									<VisibilitySensor onChange={fetchNextPage} />
								) : null}
							</>
						)}
					</section>

						<DeploymentsSection />
					</div>
				</ScrollArea>
			</div>
		);
}

function DeploymentsSection() {
	const { namespace } = useParams({ strict: false }) as { namespace: string };
	const dataProvider = useCloudNamespaceDataProvider();
	const navigate = useNavigate();

	// const {
	// 	data: images,
	// 	isError,
	// 	isLoading: isLoadingImages,
	// 	fetchNextPage,
	// 	hasNextPage,
	// } = useSuspenseInfiniteQuery({
	// 	...dataProvider.currentProjectImagesQueryOptions(),
	// 	refetchInterval: 5_000,
	// });

	// const { data: namespaces } = useSuspenseInfiniteQuery({
	// 	...dataProvider.currentProjectNamespacesQueryOptions(),
	// 	refetchInterval: 5_000,
	// });

	// const managedPoolQueries = useQueries({
	// 	queries:
	// 		namespaces.map((ns) =>
	// 			queryOptions({
	// 				...dataProvider.currentProjectManagedPoolQueryOptions({
	// 					namespace: ns.name,
	// 					pool: "default",
	// 				}),
	// 				select: (data) => ({
	// 					...data,
	// 					namespace: ns.name,
	// 					...data?.config?.image,
	// 				}),
	// 				refetchInterval: 5_000,
	// 			}),
	// 		) ?? [],
	// });

	// const deployments = managedPoolQueries
	// 	.map((query) => query.data)
	// 	.filter(
	// 		(data): data is Exclude<typeof data, undefined> =>
	// 			data !== undefined,
	// 	);

	// // Only show deployments section if there are images (namespace uses compute).
	// if (!isLoadingImages && images.length === 0) {
	// 	return null;
	// }

	// const sorted = images.toSorted((a, b) => {
	// 	const aTimestamp = new Date(a.createdAt).getTime();
	// 	const bTimestamp = new Date(b.createdAt).getTime();
	// 	return bTimestamp - aTimestamp;
	// });

	// return (
	// 	<section>
	// 		<header className="flex items-center justify-between gap-4 mb-3">
	// 			<h2 className="text-base font-semibold text-foreground">
	// 				Deployments
	// 			</h2>
	// 			<Button
	// 				variant="outline"
	// 				size="sm"
	// 				startIcon={<Icon icon={faPlus} />}
	// 				onClick={() => {
	// 					navigate({
	// 						to: ".",
	// 						search: (old) => ({
	// 							...old,
	// 							modal: "upsert-deployment",
	// 							namespace,
	// 						}),
	// 					});
	// 				}}
	// 			>
	// 				Deploy
	// 			</Button>
	// 		</header>
	// 		<div className="border rounded-md">
	// 			<ImagesTable
	// 				images={sorted}
	// 				deployments={deployments}
	// 				isLoading={isLoadingImages}
	// 				namespace={namespace}
	// 				isError={isError}
	// 				fetchNextPage={fetchNextPage}
	// 				hasNextPage={hasNextPage}
	// 			/>
	// 		</div>
	// 	</section>
	// );
	return null;
}
