import {
	faArrowUpRight,
	faLink,
	faLinkSlash,
	faSpinnerThird,
	Icon,
} from "@rivet-gg/icons";
import { Link, useMatchRoute } from "@tanstack/react-router";
import {
	type ComponentProps,
	createContext,
	type PropsWithChildren,
	type ReactNode,
	type RefObject,
	useContext,
	useLayoutEffect,
	useRef,
	useState,
} from "react";
import type { ImperativePanelGroupHandle } from "react-resizable-panels";
import { ActorBuildsList } from "@/app/actor-builds-list";
import {
	useInspectorContext,
	useInspectorEndpoint,
	useInspectorStatus,
} from "@/app/inspector-context";
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
	WithTooltip,
} from "@/components";
import { useRootLayoutOptional } from "@/components/actors/root-layout-context";
import type { HeaderLinkProps } from "@/components/header/header-link";
import { ensureTrailingSlash } from "@/lib/utils";

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
						<ConnectionStatus />
						<ScrollArea>
							<Subnav />
						</ScrollArea>
					</div>
					<div>
						<div className="border-t my-0.5 mx-2.5" />
						<div className="flex gap-0.5 my-2 px-2.5 flex-col">
							<HeaderButton asChild>
								<a
									href="https://www.rivet.dev/changelog"
									target="_blank"
									rel="noopener"
								>
									<span className="relative">
										Whats new?
										<Ping
											className="-right-4"
											data-changelog-ping
										/>
									</span>
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

const Subnav = () => {
	const matchRoute = useMatchRoute();
	const nsMatch = matchRoute({ to: "/", fuzzy: true });

	if (nsMatch === false) {
		return null;
	}

	return (
		<div className="flex gap-1.5 flex-col">
			<div className="w-full">
				<span className="block text-muted-foreground text-xs px-2 py-1 transition-colors mb-0.5">
					Instances
				</span>
				<ActorBuildsList />
			</div>
		</div>
	);
};

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

// HeaderLink is exported so route-layout.tsx can use it if needed.
export { HeaderLink };

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
