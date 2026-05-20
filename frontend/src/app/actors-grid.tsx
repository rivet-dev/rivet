import {
	faActorsBorderless,
	faGear,
	faPlus,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import {
	queryOptions,
	useInfiniteQuery,
	useQueries,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { lazy, Suspense, type ReactNode } from "react";
import {
	Button,
	cn,
	H1,
	ScrollArea,
	SmallText,
	WithTooltip,
} from "@/components";
import { useDataProvider, useCloudNamespaceDataProvider } from "@/components/actors";
import { ImagesTable } from "@/app/images-table";
import { NoProvidersAlert } from "@/components/actors/no-providers-alert";

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

function getLazyIcon(iconName: string) {
	const loader = iconModules[`../../packages/icons/dist/icons/${iconName}.js`];
	return lazy(() =>
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
}

function ActorIcon({
	iconValue,
	className,
}: {
	iconValue: string | null;
	className?: string;
}) {
	if (iconValue && isEmoji(iconValue)) {
		return <span className={cn("text-2xl", className)}>{iconValue}</span>;
	}

	const iconName = iconValue ? `fa${toPascalCase(iconValue)}` : null;

	if (!iconName) {
		return <Icon icon={faActorsBorderless} className={className} />;
	}

	const LazyIcon = getLazyIcon(iconName);
	return <LazyIcon className={className} />;
}

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

export function ActorsGrid({
	namespaceLabel,
}: {
	namespaceLabel?: string;
}) {
	const dataProvider = useDataProvider();
	const navigate = useNavigate();
	const { data, isLoading } = useInfiniteQuery(
		dataProvider.buildsQueryOptions(),
	);
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

						{!isLoading && builds.length === 0 ? (
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
												<Suspense
													fallback={
														<Icon
															icon={faActorsBorderless}
															className="opacity-60 animate-pulse"
														/>
													}
												>
													<ActorIcon
														iconValue={iconValue}
														className="text-lg"
													/>
												</Suspense>
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
							</div>
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
