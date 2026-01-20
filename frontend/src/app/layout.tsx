import { useUser } from "@clerk/clerk-react";
import {
	faArrowUpRight,
	faCog,
	faGift,
	faHome,
	faLink,
	faLinkSlash,
	faMessageSmile,
	faSpinnerThird,
	Icon,
} from "@rivet-gg/icons";
import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import {
	type ComponentProps,
	createContext,
	type PropsWithChildren,
	type ReactNode,
	type RefObject,
	Suspense,
	useContext,
	useLayoutEffect,
	useRef,
	useState,
} from "react";
import type { ImperativePanelGroupHandle } from "react-resizable-panels";
import { match } from "ts-pattern";
import {
	Button,
	type ButtonProps,
	cn,
	type ImperativePanelHandle,
	Ping,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Skeleton,
	WithTooltip,
} from "@/components";
import { useDataProvider, useDataProviderCheck } from "@/components/actors";
import { useRootLayoutOptional } from "@/components/actors/root-layout-context";
import type { HeaderLinkProps } from "@/components/header/header-link";
import { ensureTrailingSlash } from "@/lib/utils";
import { ActorBuildsList } from "./actor-builds-list";
import { Changelog } from "./changelog";
import { ContextSwitcher } from "./context-switcher";
import { HelpDropdown } from "./help-dropdown";
import {
	useInspectorContext,
	useInspectorEndpoint,
	useInspectorStatus,
} from "./inspector-context";
import { NamespaceSelect } from "./namespace-select";
import { UserDropdown } from "./user-dropdown";

interface RootProps {
	children: ReactNode;
}

const Root = ({ children }: RootProps) => {
	return <div className={cn("flex h-screen flex-col")}>{children}</div>;
};

const Main = ({
	children,
	ref,
}: RootProps & { ref?: RefObject<ImperativePanelHandle> }) => {
	return (
		<ResizablePanel ref={ref} minSize={50}>
			<main className="bg-background flex flex-1 flex-col h-full min-h-0 min-w-0 relative">
				{children}
			</main>
		</ResizablePanel>
	);
};

const SidebarDimensionsContext = createContext(0);
const SIDEBAR_MIN_WIDTH = 195; /* in px */

const VisibleInFull = ({ children }: PropsWithChildren) => {
	const groupRef = useRef<ImperativePanelGroupHandle>(null);

	const [sidebarMinWidth, setSidebarMinWidth] = useState(0);

	useLayoutEffect(() => {
		const panelGroup = document.querySelector<HTMLDivElement>(
			'[data-panel-group-id="root"]',
		);
		const resizeHandles = panelGroup?.querySelectorAll<HTMLDivElement>(
			"[data-panel-resize-handle-id]",
		);

		if (!panelGroup || !resizeHandles || resizeHandles?.length === 0) {
			return;
		}

		const observer = new ResizeObserver(() => {
			let width = panelGroup.offsetWidth;

			resizeHandles.forEach((resizeHandle) => {
				width -= resizeHandle.offsetWidth;
			});

			setSidebarMinWidth((SIDEBAR_MIN_WIDTH / width) * 100);
		});
		observer.observe(panelGroup);
		resizeHandles.forEach((resizeHandle) => {
			observer.observe(resizeHandle);
		});

		return () => {
			observer.unobserve(panelGroup);
			resizeHandles.forEach((resizeHandle) => {
				observer.unobserve(resizeHandle);
			});
			observer.disconnect();
		};
	}, []);

	return (
		// biome-ignore lint/correctness/useUniqueElementIds: id its not html element id
		<ResizablePanelGroup
			ref={groupRef}
			direction="horizontal"
			className="relative min-h-screen h-screen"
			id="root"
		>
			<SidebarDimensionsContext.Provider value={sidebarMinWidth}>
				{children}
			</SidebarDimensionsContext.Provider>
		</ResizablePanelGroup>
	);
};

export const Logo = () => {
	return (
		<Link to="/" className="flex items-center gap-5 ps-3 pt-5 pb-4">
			<img
				src={`${ensureTrailingSlash(import.meta.env.BASE_URL || "")}logo.svg`}
				alt="Rivet.gg"
				className="h-6"
			/>
		</Link>
	);
};

const Sidebar = ({
	ref,
	...props
}: {
	ref?: RefObject<ImperativePanelHandle | null>;
} & ComponentProps<typeof ResizablePanel>) => {
	const sidebarMinWidth = useContext(SidebarDimensionsContext);
	const matchRoute = useMatchRoute();
	return (
		<>
			<ResizablePanel
				ref={ref}
				minSize={sidebarMinWidth}
				maxSize={20}
				className="bg-background"
				collapsible
				{...props}
			>
				<div className="flex-col gap-2 size-full flex">
					<Logo />
					<div className="flex flex-1 flex-col gap-2 px-2 min-h-0">
						{match(__APP_TYPE__)
							.with("inspector", () => (
								<>
									<ConnectionStatus />
									<ScrollArea>
										<Subnav />
									</ScrollArea>
								</>
							))
							.with("engine", () => (
								<>
									<Breadcrumbs />
									<ScrollArea>
										<Subnav />
									</ScrollArea>
								</>
							))
							.with("cloud", () => <CloudSidebar />)
							.exhaustive()}
					</div>
					<div>
						<div className="border-t my-0.5 mx-2.5" />

						{match(__APP_TYPE__)
							.with("cloud", () => {
								return (
									<>
										<div className="flex gap-0.5 my-2 px-2.5 flex-col">
											{matchRoute({
												to: "/orgs/$organization/projects/$project/ns/$namespace",
												fuzzy: true,
											}) ? (
												<HeaderLink
													to="/orgs/$organization/projects/$project/ns/$namespace/settings"
													className="font-normal"
													icon={faCog}
												>
													Settings
												</HeaderLink>
											) : null}
											<HelpDropdown>
												<HeaderButton
													startIcon={
														<Icon
															icon={
																faMessageSmile
															}
															className="size-5 opacity-80 group-hover:opacity-100 transition-opacity"
														/>
													}
												>
													Support
												</HeaderButton>
											</HelpDropdown>

											<Changelog>
												<HeaderButton
													startIcon={
														<Icon
															icon={faGift}
															className="size-5 opacity-80 group-hover:opacity-100 transition-opacity"
														/>
													}
												>
													<a
														href="https://www.rivet.dev/changelog"
														target="_blank"
														rel="noopener"
													>
														Whats new?
														<Ping
															className="relative -right-1"
															data-changelog-ping
														/>
													</a>
												</HeaderButton>
											</Changelog>
										</div>
										<div className="border-t my-0.5 mx-2.5" />

										<div className=" px-1 pt-2 pb-4 flex flex-col">
											<UserDropdown />
										</div>
									</>
								);
							})
							.otherwise(() => (
								<>
									<div className="flex gap-0.5 my-2 px-2.5 flex-col">
										<Changelog>
											<HeaderButton asChild>
												<a
													href="https://www.rivet.dev/changelog"
													target="_blank"
													rel="noopener"
												>
													Whats new?
													<Ping
														className="relative -right-1"
														data-changelog-ping
													/>
												</a>
											</HeaderButton>
										</Changelog>
										<HeaderButton asChild>
											<Link
												to="."
												search={(old) => ({
													...old,
													modal: "feedback",
												})}
											>
												Feedback
											</Link>
										</HeaderButton>
										<HeaderButton
											asChild
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
										>
											<a
												href="https://www.rivet.dev/docs"
												target="_blank"
												rel="noopener noreferrer"
											>
												Documentation
											</a>
										</HeaderButton>
										<HeaderButton
											asChild
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
										>
											<a
												href="http://www.rivet.dev/discord"
												target="_blank"
												rel="noopener noreferrer"
											>
												Discord
											</a>
										</HeaderButton>
										<HeaderButton
											asChild
											endIcon={
												<Icon
													icon={faArrowUpRight}
													className="ms-1"
												/>
											}
										>
											<a
												href="http://github.com/rivet-dev/rivet"
												target="_blank"
												rel="noopener noreferrer"
											>
												GitHub
											</a>
										</HeaderButton>
									</div>
								</>
							))}
					</div>
				</div>
			</ResizablePanel>
			<ResizableHandle className="my-8 after:rounded-t-full after:rounded-b-full bg-transparent" />
		</>
	);
};

const Header = () => {
	return null;
};

const Footer = () => {
	return null;
};

export { Root, Main, Header, Footer, VisibleInFull, Sidebar };

const Breadcrumbs = (): ReactNode => {
	const matchRoute = useMatchRoute();
	const nsMatch = matchRoute({
		to: "/ns/$namespace",
		fuzzy: true,
	});

	if (nsMatch === false) {
		return null;
	}

	return (
		<Suspense
			fallback={
				<div className="flex items-center gap-2 ms-2 h-10">
					<Skeleton className="h-5 w-24" />
				</div>
			}
		>
			<NamespaceBreadcrumbs namespaceNameId={nsMatch.namespace} />
		</Suspense>
	);
};

const NamespaceBreadcrumbs = ({
	namespaceNameId,
}: {
	namespaceNameId: string;
}) => {
	const navigate = useNavigate();

	return (
		<div className="flex items-center gap-2">
			<NamespaceSelect
				className="text-sm py-1.5 h-auto [&>[data-icon]]:size-3"
				showCreate
				value={namespaceNameId}
				onValueChange={(value) =>
					navigate({
						to: "/ns/$namespace",
						params: {
							namespace: value,
						},
					})
				}
				onCreateClick={() =>
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							modal: "create-ns",
						}),
					})
				}
			/>
		</div>
	);
};

const Subnav = () => {
	const matchRoute = useMatchRoute();
	const nsMatch = matchRoute(
		__APP_TYPE__ === "engine"
			? {
					to: "/ns/$namespace",
					fuzzy: true,
				}
			: { to: "/", fuzzy: true },
	);

	if (nsMatch === false) {
		return null;
	}

	return (
		<div className="flex gap-1.5 flex-col">
			{__APP_TYPE__ === "engine" ? (
				<HeaderLink
					to="/ns/$namespace/connect"
					className="font-normal"
					params={nsMatch}
					icon={faHome}
				>
					Overview
				</HeaderLink>
			) : null}
			<div className="w-full">
				<span className="block text-muted-foreground text-xs px-2 py-1 transition-colors mb-0.5">
					Instances
				</span>
				<ActorBuildsList />
			</div>
		</div>
	);
};

function HeaderLink({ icon, children, className, ...props }: HeaderLinkProps) {
	return (
		<HeaderButton
			asChild
			variant="ghost"
			className="font-medium px-1 text-muted-foreground data-active:text-foreground data-active:bg-accent"
			{...props}
			startIcon={
				icon ? (
					<Icon
						className={cn(
							"size-5 opacity-80 group-hover:opacity-100 transition-opacity",
						)}
						icon={icon}
					/>
				) : undefined
			}
		>
			<Link to={props.to}>{children}</Link>
		</HeaderButton>
	);
}

function HeaderButton({ children, className, ...props }: ButtonProps) {
	return (
		<Button
			variant="ghost"
			{...props}
			className={cn(
				"text-muted-foreground px-1 aria-current-page:text-foreground relative h-auto py-1 justify-start",
				className,
			)}
		>
			{children}
		</Button>
	);
}

function ConnectionStatus(): ReactNode {
	const endpoint = useInspectorEndpoint();

	const { disconnect } = useInspectorContext();
	const status = useInspectorStatus();

	if (status === "reconnecting") {
		return (
			<div className=" border text-sm p-2 rounded-md flex items-center bg-stripes">
				<div className="flex-1">
					<p>Connecting</p>
					<p className="text-muted-foreground text-xs">{endpoint}</p>
				</div>
				<Icon icon={faSpinnerThird} className="animate-spin ml-2" />
			</div>
		);
	}

	if (status === "disconnected") {
		return (
			<div className="text-red-500 border p-2 rounded-md flex items-center text-sm justify-between bg-stripes-destructive ">
				<div className="flex items-center">
					<div>
						<p>Disconnected</p>
						<p className="text-muted-foreground text-xs">
							{endpoint}
						</p>
					</div>
				</div>

				<WithTooltip
					delayDuration={0}
					trigger={
						<Button
							variant="outline"
							size="icon-sm"
							className="ml-2 text-foreground"
							onClick={() => disconnect()}
						>
							<Icon icon={faLink} />
						</Button>
					}
					content="Reconnect"
				/>
			</div>
		);
	}

	if (status === "connected") {
		return (
			<div className=" border text-sm p-2 rounded-md flex items-center bg-stripes justify-between">
				<div>
					<p>Connected</p>
					<p className="text-muted-foreground text-xs">{endpoint}</p>
				</div>

				<WithTooltip
					delayDuration={0}
					trigger={
						<Button
							variant="outline"
							size="icon-sm"
							className="ml-2 text-foreground"
							onClick={() => disconnect()}
						>
							<Icon icon={faLinkSlash} />
						</Button>
					}
					content="Disconnect"
				/>
			</div>
		);
	}

	return null;
}

function CloudSidebar(): ReactNode {
	return (
		<>
			<ContextSwitcher />

			<ScrollArea>
				<CloudSidebarContent />
			</ScrollArea>
		</>
	);
}

function CloudSidebarContent() {
	const match = useMatchRoute();

	const matchNamespace = match({
		to: "/orgs/$organization/projects/$project/ns/$namespace",
		fuzzy: true,
	});

	if (matchNamespace) {
		return <CloudSidebarContentInner />;
	}

	return null;
}

function CloudSidebarContentInner() {
	const hasDataProvider = useDataProviderCheck();
	const hasQuery = !!useDataProvider().buildsQueryOptions;
	return (
		<div className="flex gap-0.5 flex-col">
			{hasDataProvider && hasQuery ? (
				<div className="w-full pt-1.5">
					<span className="block text-muted-foreground text-xs px-2 py-1 transition-colors mb-0.5">
						Actors
					</span>
					<ActorBuildsList />
				</div>
			) : null}
		</div>
	);
}

export const Content = ({
	className,
	children,
}: {
	className?: string;
	children: ReactNode;
}) => {
	const isInRootLayout = !!useRootLayoutOptional();
	const { isSidebarCollapsed } = useRootLayoutOptional() || {};
	return (
		<div
			className={cn(
				" h-full overflow-auto @container transition-colors",
				!isSidebarCollapsed &&
					isInRootLayout &&
					"border my-2 bg-card rounded-lg mr-2",
				!isInRootLayout && "h-screen",
				className,
			)}
		>
			{children}
		</div>
	);
};

export const SidebarlessHeader = () => {
	const { user } = useUser();
	return (
		<div className="rounded-lg flex items-center pe-1.5 justify-between bg-card/10 backdrop-blur-lg fixed inset-x-0 top-0 z-10">
			<Logo />

			<div className="flex gap-4">
				<ContextSwitcher inline />
				<UserDropdown>
					<Button
						variant="ghost"
						className="text-sm text-muted-foreground font-normal px-1.5"
					>
						Logged in as{" "}
						<span className="text-foreground">
							{user?.primaryEmailAddress?.emailAddress}
						</span>
					</Button>
				</UserDropdown>
			</div>
		</div>
	);
};
