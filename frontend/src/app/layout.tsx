import { useClerk } from "@clerk/clerk-react";
import {
	faArrowUpRight,
	faBolt,
	faLink,
	faServer,
	faSpinnerThird,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import {
	Link,
	useMatchRoute,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
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
	DocsSheet,
	type ImperativePanelHandle,
	Ping,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Skeleton,
} from "@/components";
import { useInspectorDataProvider } from "@/components/actors";
import type { HeaderLinkProps } from "@/components/header/header-link";
import { ensureTrailingSlash } from "@/lib/utils";
import { ActorBuildsList } from "./actor-builds-list";
import { Changelog } from "./changelog";
import { ContextSwitcher } from "./context-switcher";
import { useInspectorCredentials } from "./credentials-context";
import { HelpDropdown } from "./help-dropdown";
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

const Sidebar = ({
	ref,
	...props
}: {
	ref?: RefObject<ImperativePanelHandle>;
} & ComponentProps<typeof ResizablePanel>) => {
	const sidebarMinWidth = useContext(SidebarDimensionsContext);
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
					<Link
						to="/"
						className="flex items-center gap-2 ps-3 pt-5 pb-4"
					>
						<img
							src={`${ensureTrailingSlash(import.meta.env.BASE_URL || "")}logo.svg`}
							alt="Rivet.gg"
							className="h-6"
						/>
					</Link>
					<div className="flex flex-1 flex-col gap-4 px-2 min-h-0">
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
						<div
							className={cn(
								"px-1 py-1 flex flex-col gap-[1px] text-sm ",
								__APP_TYPE__ !== "cloud" ? "pb-4" : "",
							)}
						>
							<Changelog>
								<Button
									className="text-muted-foreground justify-start py-1 h-auto"
									variant="ghost"
									size="xs"
									asChild
								>
									<a
										href="https://rivet.gg/changelog"
										target="_blank"
										rel="noopener"
									>
										Whats new?
										<Ping
											className="relative -right-1"
											data-changelog-ping
										/>
									</a>
								</Button>
							</Changelog>
							{match(__APP_TYPE__)
								.with("cloud", () => (
									<Button
										className="text-muted-foreground justify-start py-1 h-auto"
										variant="ghost"
										size="xs"
										onClick={() => {
											Plain.open();
										}}
									>
										Feedback & Support
									</Button>
								))
								.otherwise(() => (
									<Button
										className="text-muted-foreground justify-start py-1 h-auto"
										variant="ghost"
										size="xs"
										asChild
									>
										<Link
											to="."
											search={(old) => ({
												...old,
												modal: "feedback",
											})}
										>
											Feedback
										</Link>
									</Button>
								))}
							<DocsSheet
								path={"https://rivet.gg/docs"}
								title="Documentation"
							>
								<Button
									className="text-muted-foreground justify-start py-1 h-auto"
									variant="ghost"
									size="xs"
								>
									Documentation
								</Button>
							</DocsSheet>
							<Button
								variant="ghost"
								className="text-muted-foreground justify-start py-1 h-auto"
								size="xs"
								endIcon={
									<Icon
										icon={faArrowUpRight}
										className="ms-1"
									/>
								}
							>
								<a
									href="http://rivet.gg/discord"
									target="_blank"
									rel="noopener noreferrer"
								>
									Discord
								</a>
							</Button>
							<Button
								variant="ghost"
								size="xs"
								className="text-muted-foreground justify-start py-1 h-auto"
								endIcon={
									<Icon
										icon={faArrowUpRight}
										className="ms-1"
									/>
								}
							>
								<a
									href="http://github.com/rivet-gg/rivet"
									target="_blank"
									rel="noopener noreferrer"
								>
									GitHub
								</a>
							</Button>
						</div>
						{match(__APP_TYPE__)
							.with("cloud", () => (
								<>
									<div className="border-t my-0.5 mx-2.5" />

									<div className=" px-1 pt-2 pb-4 flex flex-col">
										<CloudSidebarFooter />
									</div>
								</>
							))
							.otherwise(() => null)}
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
				onValueChange={(value) => {
					navigate({
						to: "/ns/$namespace",
						params: {
							namespace: value,
						},
					});
				}}
				onCreateClick={() => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							modal: "create-ns",
						}),
					});
				}}
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
					icon={faBolt}
				>
					Connect
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
			className="font-medium px-1 text-foreground data-active:bg-accent"
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
				"text-muted-foreground px-2 aria-current-page:text-foreground relative h-auto py-1 justify-start",
				className,
			)}
		>
			{children}
		</Button>
	);
}

function ConnectionStatus(): ReactNode {
	const endpoint = useSearch({
		from: "/_context",
		select: (s) => s.u,
	});
	const data = useInspectorDataProvider();
	const { setCredentials } = useInspectorCredentials();
	const { isLoading, isError, isSuccess } = useQuery(
		data.statusQueryOptions(),
	);

	if (isLoading) {
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

	if (isError) {
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

				<Button
					variant="outline"
					size="xs"
					className="ml-2 text-foreground"
					onClick={() => setCredentials(null)}
					startIcon={<Icon icon={faLink} />}
				>
					Reconnect
				</Button>
			</div>
		);
	}

	if (isSuccess) {
		return (
			<div className=" border text-sm p-2 rounded-md flex items-center bg-stripes">
				<div>
					<p>Connected</p>
					<p className="text-muted-foreground text-xs">{endpoint}</p>
				</div>
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
		return (
			<div className="flex gap-1.5 flex-col">
				<HeaderLink
					to="/orgs/$organization/projects/$project/ns/$namespace/connect"
					className="font-normal"
					params={matchNamespace}
					icon={faBolt}
				>
					Connect
				</HeaderLink>
				<div className="w-full pt-1.5">
					<span className="block text-muted-foreground text-xs px-1 py-1 transition-colors mb-0.5">
						Instances
					</span>
					<ActorBuildsList />
				</div>
			</div>
		);
	}

	return null;
}

function CloudSidebarFooter() {
	return <UserDropdown />;
}
