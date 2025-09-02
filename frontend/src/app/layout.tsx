import {
	faArrowUpRight,
	faCheck,
	faLink,
	faServer,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
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
import {
	Button,
	cn,
	DocsSheet,
	type ImperativePanelHandle,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Skeleton,
} from "@/components";
import { useManager } from "@/components/actors";
import type { HeaderLinkProps } from "@/components/header/header-link";
import { ensureTrailingSlash } from "@/lib/utils";
import type { NamespaceNameId } from "@/queries/manager-engine";
import { ActorBuildsList } from "./actor-builds-list";
import { useInspectorCredentials } from "./credentials-context";
import { NamespaceSelect } from "./namespace-select";

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
						{__APP_TYPE__ === "inspector" ? (
							<ConnectionStatus />
						) : null}
						{__APP_TYPE__ === "engine" ? <Breadcrumbs /> : null}
						<ScrollArea>
							<Subnav />
						</ScrollArea>
					</div>
					<div>
						<div className="border-t p-2 flex flex-col gap-[1px] text-sm">
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
								<a href="http://rivet.gg/discord">Discord</a>
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
								<a href="http://github.com/rivet-gg/rivet">
									GitHub
								</a>
							</Button>
						</div>
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

const Breadcrumbs = () => {
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
			<NamespaceBreadcrumbs
				namespaceNameId={nsMatch?.namespace as NamespaceNameId}
			/>
		</Suspense>
	);
};

const NamespaceBreadcrumbs = ({
	namespaceNameId,
}: {
	namespaceNameId: NamespaceNameId;
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
					to="/ns/$namespace/runners"
					className="font-normal"
					params={nsMatch}
					icon={faServer}
				>
					Runners
				</HeaderLink>
			) : null}
			<div className="w-full">
				<span className="block text-muted-foreground text-xs px-2 py-1 transition-colors">
					Instances
				</span>
				<ActorBuildsList />
			</div>
		</div>
	);
};

function HeaderLink({ icon, children, className, ...props }: HeaderLinkProps) {
	return (
		<Button
			asChild
			variant="ghost"
			{...props}
			className={cn(
				"text-muted-foreground px-2 aria-current-page:text-foreground relative h-auto py-1 justify-start",
				className,
			)}
			startIcon={
				icon ? (
					<Icon className={cn("size-5 opacity-80")} icon={icon} />
				) : undefined
			}
		>
			<Link to={props.to}>{children}</Link>
		</Button>
	);
}

function ConnectionStatus() {
	const { endpoint, ...queries } = useManager();
	const { setCredentials } = useInspectorCredentials();
	const { isLoading, isError, isSuccess } = useQuery(
		queries.managerStatusQueryOptions(),
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
}
