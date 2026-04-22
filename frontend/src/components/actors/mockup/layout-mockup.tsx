import {
	faActorsBorderless,
	faArrowUp,
	faChevronDown,
	faCircleUser,
	faCodeBranch,
	faComment,
	faMagnifyingGlass,
	faMoon,
	faPlus,
	faRivet,
	faSliders,
	faSun,
	faXmark,
	Icon,
} from "@rivet-gg/icons";
import {
	Background,
	BackgroundVariant,
	BaseEdge,
	Controls,
	type Edge,
	EdgeLabelRenderer,
	type EdgeProps,
	type EdgeTypes,
	getSmoothStepPath,
	Handle,
	MarkerType,
	type Node,
	type NodeChange,
	type NodeProps,
	type NodeTypes,
	type OnConnect,
	Position,
	ReactFlow,
	ReactFlowProvider,
	addEdge,
	applyEdgeChanges,
	applyNodeChanges,
	type EdgeChange,
	useReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { javascript } from "@codemirror/lang-javascript";
import {
	githubDarkInit,
	githubLightInit,
} from "@uiw/codemirror-theme-github";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
import {
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";
import {
	Button,
	cn,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/components";
import { CodeMirror } from "@/components/code-mirror";
import { ActorStatusIndicator } from "../actor-status-indicator";
import { useDataProvider } from "../data-provider";
import type { ActorId, ActorStatus } from "../queries";

// -- Theme --

type Theme = "light" | "dark";
const THEME_STORAGE_KEY = "mockup-theme";

function readStoredTheme(): Theme {
	if (typeof window === "undefined") return "dark";
	const stored = localStorage.getItem(THEME_STORAGE_KEY);
	if (stored === "light" || stored === "dark") return stored;
	return document.documentElement.classList.contains("dark")
		? "dark"
		: "light";
}

let currentTheme: Theme =
	typeof window === "undefined" ? "dark" : readStoredTheme();
const themeListeners = new Set<() => void>();

function applyTheme(next: Theme) {
	currentTheme = next;
	document.documentElement.classList.toggle("dark", next === "dark");
	localStorage.setItem(THEME_STORAGE_KEY, next);
	for (const listener of themeListeners) listener();
}

function subscribeTheme(listener: () => void): () => void {
	themeListeners.add(listener);
	return () => {
		themeListeners.delete(listener);
	};
}

function getThemeSnapshot(): Theme {
	return currentTheme;
}

function useTheme(): [Theme, () => void] {
	const theme = useSyncExternalStore(
		subscribeTheme,
		getThemeSnapshot,
		getThemeSnapshot,
	);

	// Ensure the DOM reflects the stored preference on first render.
	useEffect(() => {
		if (typeof window === "undefined") return;
		const isDark = document.documentElement.classList.contains("dark");
		if ((currentTheme === "dark") !== isDark) {
			applyTheme(currentTheme);
		}
	}, []);

	const toggle = useCallback(() => {
		applyTheme(currentTheme === "dark" ? "light" : "dark");
	}, []);

	return [theme, toggle];
}

// -- Top Bar --

function MockupTopBar() {
	return (
		<div className="h-12 mt-2 mx-2 border dark:border-white/10 rounded-lg bg-card flex items-center justify-between px-3 shrink-0 z-20">
			<div className="flex items-center gap-3">
				<Icon icon={faRivet} className="text-2xl text-foreground" />
				<div className="h-5 w-px bg-border" />
				<NamespaceDropdownPlaceholder />
			</div>

			<div className="flex items-center gap-1">
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground"
				>
					Actor Builds
				</Button>
				<div className="h-5 w-px bg-border mx-1" />
				<AccountMenu />
			</div>
		</div>
	);
}

function AccountMenu() {
	const [theme, toggleTheme] = useTheme();

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground gap-2"
				>
					<Icon icon={faCircleUser} className="text-base" />
					<span className="text-xs">Company</span>
					<Icon
						icon={faChevronDown}
						className="text-[10px] opacity-60"
					/>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="w-48">
				<DropdownMenuItem>Billing</DropdownMenuItem>
				<DropdownMenuItem>Support</DropdownMenuItem>
				<DropdownMenuItem>What's New</DropdownMenuItem>
				<DropdownMenuSeparator />
				<DropdownMenuItem
					onSelect={(e) => {
						e.preventDefault();
						toggleTheme();
					}}
				>
					<Icon
						icon={theme === "dark" ? faSun : faMoon}
						className="mr-2 w-3.5 text-muted-foreground"
					/>
					{theme === "dark" ? "Light mode" : "Dark mode"}
				</DropdownMenuItem>
				<DropdownMenuItem>Settings</DropdownMenuItem>
				<DropdownMenuItem>Sign out</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function NamespaceDropdownPlaceholder() {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className="text-foreground gap-2 font-medium"
				>
					<span className="text-xs">Namespace</span>
					<Icon
						icon={faChevronDown}
						className="text-[10px] opacity-60"
					/>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="start" className="w-48">
				<DropdownMenuItem>default</DropdownMenuItem>
				<DropdownMenuItem>staging</DropdownMenuItem>
				<DropdownMenuItem>production</DropdownMenuItem>
				<DropdownMenuSeparator />
				<DropdownMenuItem>New namespace</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

// -- Actor Node for Canvas --

interface ActorNodeData {
	actorId: ActorId;
	label: string;
	instances: number;
	status: ActorStatus;
	version: string;
	[key: string]: unknown;
}

function ActorCanvasNode({ data }: NodeProps<Node<ActorNodeData>>) {
	return (
		<div className="bg-card border dark:border-white/10 dark:bg-white/[0.02] rounded-lg px-4 py-3 w-[240px] cursor-pointer hover:border-primary/50 transition-colors shadow-sm relative group">
			<Handle
				type="source"
				position={Position.Top}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-0 group-hover:opacity-100 transition-opacity"
			/>
			<Handle
				type="source"
				position={Position.Right}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-0 group-hover:opacity-100 transition-opacity"
				id="right"
			/>
			<Handle
				type="target"
				position={Position.Bottom}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-0 group-hover:opacity-100 transition-opacity"
				id="bottom"
			/>
			<Handle
				type="target"
				position={Position.Left}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-0 group-hover:opacity-100 transition-opacity"
				id="left"
			/>
			<div className="flex items-center gap-2">
				<Icon
					icon={faActorsBorderless}
					className="text-muted-foreground shrink-0"
				/>
				<div className="min-w-0 flex-1">
					<div className="text-sm truncate">{data.label}</div>
					<div className="text-[10px] text-muted-foreground flex items-center gap-1.5">
						<span className="font-mono">{data.version}</span>
						<span className="opacity-40">·</span>
						<span>
							{data.instances} instance
							{data.instances !== 1 ? "s" : ""}
						</span>
					</div>
				</div>
				<div className="flex items-center shrink-0">
					<ActorStatusIndicator status={data.status} />
				</div>
			</div>
		</div>
	);
}

const nodeTypes: NodeTypes = {
	actor: ActorCanvasNode,
};

// -- Deletable Edge --

function DeletableEdge({
	id,
	sourceX,
	sourceY,
	targetX,
	targetY,
	sourcePosition,
	targetPosition,
	style,
	markerEnd,
	selected,
}: EdgeProps) {
	const { setEdges } = useReactFlow();
	const [edgePath, labelX, labelY] = getSmoothStepPath({
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
	});

	const handleDelete = useCallback(
		(event: React.MouseEvent) => {
			event.stopPropagation();
			setEdges((edges) => edges.filter((edge) => edge.id !== id));
		},
		[id, setEdges],
	);

	return (
		<>
			<BaseEdge path={edgePath} style={style} markerEnd={markerEnd} />
			{selected ? (
				<EdgeLabelRenderer>
					<div
						className="nodrag nopan absolute pointer-events-auto"
						style={{
							transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
						}}
					>
						<button
							type="button"
							onClick={handleDelete}
							className="flex items-center justify-center w-5 h-5 rounded-full bg-background border text-muted-foreground hover:text-destructive hover:border-destructive shadow-sm transition-colors"
							aria-label="Delete edge"
						>
							<Icon icon={faXmark} className="w-2.5" />
						</button>
					</div>
				</EdgeLabelRenderer>
			) : null}
		</>
	);
}

const edgeTypes: EdgeTypes = {
	deletable: DeletableEdge,
};

// -- Canvas Auto-Fit Hook --

function FitViewOnChange({
	leftOpen,
	rightOpen,
}: { leftOpen: boolean; rightOpen: boolean }) {
	const { fitView } = useReactFlow();
	const isFirstRender = useRef(true);

	useEffect(() => {
		if (isFirstRender.current) {
			isFirstRender.current = false;
			return;
		}
		// Wait for CSS transition to finish before fitting
		const timer = setTimeout(() => fitView({ duration: 300 }), 350);
		return () => clearTimeout(timer);
	}, [leftOpen, rightOpen, fitView]);

	return null;
}

// -- Canvas State Persistence --

const STORAGE_KEY = "mockup-canvas-state";

const PLACEHOLDERS: {
	name: string;
	instances: number;
	status: ActorStatus;
	version: string;
}[] = [
	{ name: "Leaderboard", instances: 3, status: "running", version: "v12" },
	{ name: "Chat Room", instances: 12, status: "running", version: "v8" },
	{ name: "Game Lobby", instances: 1, status: "sleeping", version: "v4" },
	{ name: "Auth Session", instances: 48, status: "running", version: "v21" },
	{ name: "Match Maker", instances: 5, status: "starting", version: "v2" },
	{ name: "Inventory", instances: 7, status: "running", version: "v15" },
	{ name: "Player Stats", instances: 1, status: "crashed", version: "v3" },
	{ name: "World State", instances: 2, status: "running", version: "v9" },
];

function loadCanvasState(): {
	nodes: Node<ActorNodeData>[];
	edges: Edge[];
} | null {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (raw) return JSON.parse(raw);
	} catch {
		// ignore
	}
	return null;
}

function saveCanvasState(nodes: Node<ActorNodeData>[], edges: Edge[]) {
	try {
		localStorage.setItem(
			STORAGE_KEY,
			JSON.stringify({
				nodes: nodes.map((n) => ({
					...n,
					selected: false,
				})),
				edges,
			}),
		);
	} catch {
		// ignore
	}
}

function makeDefaultNodes(): Node<ActorNodeData>[] {
	const COLS = 4;
	const X_GAP = 280;
	const Y_GAP = 100;

	return PLACEHOLDERS.map((placeholder, i) => ({
		id: `placeholder-${i}`,
		type: "actor",
		position: {
			x: (i % COLS) * X_GAP,
			y: Math.floor(i / COLS) * Y_GAP,
		},
		data: {
			actorId: `placeholder-${i}` as ActorId,
			label: placeholder.name,
			instances: placeholder.instances,
			status: placeholder.status,
			version: placeholder.version,
		},
	}));
}

// -- Actor Canvas --

function ActorCanvas({
	onActorClick,
	leftOpen,
	rightOpen,
}: {
	onActorClick: (actorId: string) => void;
	leftOpen: boolean;
	rightOpen: boolean;
}) {
	const n = useSearch({
		from: "/_context",
		select: (state) => state.n,
	});
	const { data: actors = [] } = useInfiniteQuery(
		useDataProvider().actorsListQueryOptions({ n }),
	);

	const [nodes, setNodes] = useState<Node<ActorNodeData>[]>(() => {
		const saved = loadCanvasState();
		if (saved) return saved.nodes ?? [];
		return makeDefaultNodes();
	});

	const [edges, setEdges] = useState<Edge[]>(() => {
		const saved = loadCanvasState();
		return (saved?.edges ?? []).map((edge) => ({
			...edge,
			type: "deletable",
		}));
	});

	// Update nodes from real actors if available
	useEffect(() => {
		if (actors.length === 0) return;
		const COLS = 4;
		const X_GAP = 280;
		const Y_GAP = 100;

		setNodes(
			actors.map((actor, i) => ({
				id: actor.actorId,
				type: "actor",
				position: {
					x: (i % COLS) * X_GAP,
					y: Math.floor(i / COLS) * Y_GAP,
				},
				data: {
					actorId: actor.actorId,
					label: actor.key || actor.actorId.substring(0, 8),
					instances: 1,
					status: "running",
					version: "v1",
				},
			})),
		);
	}, [actors]);

	// Persist state on changes
	const saveTimeout = useRef<ReturnType<typeof setTimeout> | undefined>(
		undefined,
	);
	useEffect(() => {
		clearTimeout(saveTimeout.current);
		saveTimeout.current = setTimeout(() => saveCanvasState(nodes, edges), 500);
		return () => clearTimeout(saveTimeout.current);
	}, [nodes, edges]);

	const onNodesChange = useCallback(
		(changes: NodeChange<Node<ActorNodeData>>[]) => {
			setNodes((nds) => applyNodeChanges(changes, nds));
		},
		[],
	);

	const onEdgesChange = useCallback(
		(changes: EdgeChange[]) => {
			setEdges((eds) => applyEdgeChanges(changes, eds));
		},
		[],
	);

	const onConnect: OnConnect = useCallback(
		(connection) => {
			setEdges((eds) => addEdge(connection, eds));
		},
		[],
	);

	const handleNodeClick = useCallback(
		(_event: React.MouseEvent, node: Node) => {
			onActorClick(node.id);
		},
		[onActorClick],
	);

	const handlePaneClick = useCallback(() => {
		onActorClick("");
	}, [onActorClick]);

	return (
		<ReactFlowProvider>
			<FitViewOnChange leftOpen={leftOpen} rightOpen={rightOpen} />
			<ReactFlow
				nodes={nodes}
				edges={edges}
				nodeTypes={nodeTypes}
				edgeTypes={edgeTypes}
				onNodesChange={onNodesChange}
				onEdgesChange={onEdgesChange}
				onConnect={onConnect}
				fitView
				panOnScroll
				panOnDrag
				nodesDraggable
				nodesConnectable
				edgesReconnectable
				onNodeClick={handleNodeClick}
				onPaneClick={handlePaneClick}
				proOptions={{ hideAttribution: true }}
				deleteKeyCode={["Backspace", "Delete"]}
				defaultEdgeOptions={{
					style: { stroke: "hsl(var(--muted-foreground))", strokeWidth: 1.5 },
					type: "deletable",
					markerEnd: {
						type: MarkerType.ArrowClosed,
						color: "hsl(var(--muted-foreground))",
					},
				}}
			>
				<Background
					variant={BackgroundVariant.Dots}
					gap={20}
					size={1.8}
					color="hsl(var(--muted-foreground) / 0.35)"
				/>
				<Controls />
			</ReactFlow>
			{nodes.length === 0 ? <CanvasEmptyState /> : null}
		</ReactFlowProvider>
	);
}

function CanvasEmptyState() {
	return (
		<div className="absolute inset-0 flex items-center justify-center pointer-events-none z-[5]">
			<div className="flex flex-col items-center gap-4 pointer-events-auto">
				<div className="w-20 h-20 rounded-2xl border-2 border-dashed border-muted-foreground/30 flex items-center justify-center">
					<Icon
						icon={faActorsBorderless}
						className="text-3xl text-muted-foreground/60"
					/>
				</div>
				<div className="text-center max-w-xs">
					<h3 className="text-sm font-medium text-foreground">
						No actors yet
					</h3>
					<p className="text-xs text-muted-foreground mt-1">
						Create your first actor to start building.
					</p>
				</div>
				<Button size="sm" className="gap-1.5 text-xs">
					<Icon icon={faPlus} className="w-3" />
					Create actor
				</Button>
			</div>
		</div>
	);
}

// -- Left Popover (Agent / Versions) --

function LeftPopover({
	tab,
	onTabChange,
	onClose,
	visible,
}: {
	tab: "agent" | "versions";
	onTabChange: (tab: "agent" | "versions") => void;
	onClose: () => void;
	visible: boolean;
}) {
	return (
		<div
			className={cn(
				"absolute left-4 top-4 bottom-4 w-80 bg-card border dark:border-white/10 rounded-lg shadow-xl z-10 flex flex-col overflow-hidden transition-transform duration-300 ease-out",
				visible
					? "translate-x-0"
					: "-translate-x-[calc(100%+2rem)]",
			)}
		>
			<Tabs
				value={tab}
				onValueChange={(v) => onTabChange(v as "agent" | "versions")}
				className="flex flex-col flex-1 min-h-0"
			>
				<div className="flex items-center justify-between border-b h-[45px] px-2">
					<TabsList className="items-center bg-transparent gap-0.5 border-b-0 h-auto w-auto">
						<TabsTrigger
							value="agent"
							className="text-xs px-2.5 py-1 h-7 min-h-0 rounded-md font-medium border-b-0 data-[state=active]:border-b-transparent data-[state=active]:bg-accent"
						>
							Agent
						</TabsTrigger>
						<TabsTrigger
							value="versions"
							className="text-xs px-2.5 py-1 h-7 min-h-0 rounded-md font-medium border-b-0 data-[state=active]:border-b-transparent data-[state=active]:bg-accent"
						>
							Versions
						</TabsTrigger>
					</TabsList>
					<Button variant="ghost" size="icon-sm" onClick={onClose}>
						<Icon icon={faXmark} />
					</Button>
				</div>
				<TabsContent value="agent" className="flex-1 mt-0 min-h-0">
					<AgentChat />
				</TabsContent>
				<TabsContent value="versions" className="flex-1 mt-0 min-h-0">
					<ScrollArea className="h-full p-4">
						<div className="text-muted-foreground text-sm text-center py-8">
							Versions panel placeholder
						</div>
					</ScrollArea>
				</TabsContent>
			</Tabs>
		</div>
	);
}

// -- Fake Agent Chat UI --

const FAKE_MESSAGES = [
	{
		role: "user" as const,
		text: "Add a leaderboard system to the game",
	},
	{
		role: "assistant" as const,
		text: "I'll add a leaderboard actor that tracks high scores. I'm creating a new actor with KV storage for scores and RPC actions to submit and query them.",
	},
	{
		role: "user" as const,
		text: "Make it show the top 10 players",
	},
	{
		role: "assistant" as const,
		text: "Done. The `getTopScores` action now returns the top 10 sorted by score descending. I also added a `submitScore` action that only updates if the new score is higher.",
	},
];

function AgentChat() {
	return (
		<div className="flex flex-col h-full min-h-0">
			<ScrollArea className="flex-1">
				<div className="px-4 py-4 space-y-5">
					{FAKE_MESSAGES.map((msg, i) =>
						msg.role === "user" ? (
							<div key={i} className="flex justify-end">
								<div className="rounded-2xl rounded-br-md bg-accent text-foreground text-xs leading-relaxed px-3 py-2 max-w-[85%]">
									{msg.text}
								</div>
							</div>
						) : (
							<div
								key={i}
								className="text-xs leading-relaxed text-foreground"
							>
								{msg.text}
							</div>
						),
					)}
				</div>
			</ScrollArea>
			<div className="p-3">
				<div className="flex items-center gap-2 rounded-lg border bg-muted/20 focus-within:bg-muted/40 focus-within:border-foreground/30 transition-colors px-2.5 py-1">
					<input
						type="text"
						placeholder="Ask the agent..."
						className="flex-1 min-w-0 bg-transparent text-xs placeholder:text-muted-foreground focus:outline-none py-1.5"
					/>
					<button
						type="button"
						aria-label="Send message"
						className="w-6 h-6 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent flex items-center justify-center transition-colors shrink-0"
					>
						<Icon icon={faArrowUp} className="w-2.5" />
					</button>
				</div>
			</div>
		</div>
	);
}

// -- Left Floating Buttons --

function LeftFloatingButtons({
	onOpen,
	visible,
}: {
	onOpen: (tab: "agent" | "versions") => void;
	visible: boolean;
}) {
	return (
		<div
			className={cn(
				"absolute left-4 top-4 z-10 bg-card border dark:border-white/10 rounded-md overflow-hidden flex flex-col transition-all duration-300 ease-out",
				visible
					? "translate-x-0 opacity-100"
					: "-translate-x-4 opacity-0 pointer-events-none",
			)}
		>
			<button
				type="button"
				onClick={() => onOpen("agent")}
				className="flex items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent border-b border-border text-foreground transition-colors"
			>
				<Icon
					icon={faComment}
					className="text-muted-foreground w-3.5"
				/>
				<span>Agent</span>
			</button>
			<button
				type="button"
				onClick={() => onOpen("versions")}
				className="flex items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent text-foreground transition-colors"
			>
				<Icon
					icon={faCodeBranch}
					className="text-muted-foreground w-3.5"
				/>
				<span>Versions</span>
			</button>
		</div>
	);
}

// -- Right Floating Buttons --

function RightFloatingButtons({ visible }: { visible: boolean }) {
	return (
		<div
			className={cn(
				"absolute right-4 top-4 z-10 bg-card border dark:border-white/10 rounded-md overflow-hidden flex flex-col transition-all duration-300 ease-out",
				visible
					? "translate-x-0 opacity-100"
					: "translate-x-4 opacity-0 pointer-events-none",
			)}
		>
			<button
				type="button"
				className="flex items-center gap-2 px-2.5 py-1.5 text-xs hover:bg-accent text-foreground transition-colors"
			>
				<Icon icon={faPlus} className="text-muted-foreground w-3.5" />
				<span>Create actor</span>
			</button>
		</div>
	);
}

// -- Right Popover (Actor List + Details) --

function RightPopover({
	actorId,
	onClose,
	onSelectActor,
	visible,
}: {
	actorId: string;
	onClose: () => void;
	onSelectActor: (actorId: string) => void;
	visible: boolean;
}) {
	return (
		<div
			className={cn(
				"absolute right-4 top-4 bottom-4 w-[48%] min-w-[560px] max-w-[1100px] bg-card border dark:border-white/10 rounded-lg shadow-xl z-10 flex flex-col overflow-hidden transition-transform duration-300 ease-out",
				visible
					? "translate-x-0"
					: "translate-x-[calc(100%+2rem)]",
			)}
		>
			<ResizablePanelGroup direction="horizontal" className="flex-1">
				<ResizablePanel
					defaultSize={32}
					minSize={20}
					maxSize={50}
					className="flex flex-col"
				>
					<ActorListToolbar />
					<ActorListPlaceholder
						selectedActorId={actorId}
						onSelect={onSelectActor}
					/>
				</ResizablePanel>
				<ResizableHandle />
				<ResizablePanel defaultSize={68} minSize={50} className="flex flex-col min-w-0">
					<ActorDetailPlaceholder actorId={actorId} onClose={onClose} />
				</ResizablePanel>
			</ResizablePanelGroup>
		</div>
	);
}

function ActorListToolbar() {
	return (
		<>
			<div className="flex items-center border-b h-[37px] px-3">
				<span className="text-xs font-medium text-foreground">
					Actors
				</span>
				<span className="ml-auto text-[10px] text-muted-foreground tabular-nums">
					{PLACEHOLDERS.length} total
				</span>
			</div>
			<div className="flex items-center justify-end gap-0.5 border-b h-[34px] px-1 bg-muted/30">
				<Button variant="ghost" size="icon-sm" className="h-7 w-7">
					<Icon icon={faMagnifyingGlass} />
				</Button>
				<Button variant="ghost" size="icon-sm" className="h-7 w-7">
					<Icon icon={faPlus} />
				</Button>
				<Button variant="ghost" size="icon-sm" className="h-7 w-7">
					<Icon icon={faSliders} />
				</Button>
			</div>
		</>
	);
}

function ActorListPlaceholder({
	selectedActorId,
	onSelect,
}: {
	selectedActorId: string;
	onSelect: (actorId: string) => void;
}) {
	return (
		<ScrollArea className="flex-1">
			{PLACEHOLDERS.map((placeholder, i) => {
				const id = `placeholder-${i}`;
				const isSelected = id === selectedActorId;
				return (
					<button
						type="button"
						key={id}
						onClick={() => onSelect(id)}
						className={cn(
							"w-full flex items-center gap-2 pl-2 pr-2.5 py-1 border-b border-l-2 border-l-transparent text-sm cursor-pointer hover:bg-accent/50 text-left",
							isSelected && "bg-accent border-l-primary",
						)}
					>
						<ActorStatusIndicator status={placeholder.status} />
						<span className="text-xs truncate flex-1 min-w-0">
							{placeholder.name}
						</span>
						<span className="text-[10px] text-muted-foreground font-mono shrink-0">
							{placeholder.version}
						</span>
					</button>
				);
			})}
			<button
				type="button"
				className="w-full flex items-center gap-2 pl-2 pr-2.5 py-1 border-b border-dashed border-l-2 border-l-transparent text-muted-foreground hover:text-foreground hover:bg-accent/30 transition-colors"
			>
				<span className="size-2 rounded-full border border-dashed border-muted-foreground/60" />
				<span className="text-xs flex-1 text-left">Create actor</span>
				<Icon icon={faPlus} className="w-2.5 opacity-60" />
			</button>
		</ScrollArea>
	);
}

// -- Code Tab --

const EXAMPLE_CODE = `import { Actor } from "rivetkit";

export default class Leaderboard extends Actor {
  override initialize() {
    return { scores: new Map() };
  }

  async submitScore(playerName: string, score: number) {
    const current = this.state.scores.get(playerName) ?? 0;
    if (score > current) {
      this.state.scores.set(playerName, score);
    }
  }

  async getTopScores() {
    return [...this.state.scores.entries()]
      .sort(([, a], [, b]) => b - a)
      .slice(0, 10)
      .map(([name, score]) => ({ name, score }));
  }
}
`;

const TAB_GROUPS = [
	{
		id: "source",
		label: "Source",
		tabs: [
			{ value: "code", label: "Code" },
			{ value: "workflow", label: "Workflow" },
		],
	},
	{
		id: "data",
		label: "Data",
		tabs: [
			{ value: "database", label: "Database" },
			{ value: "state", label: "State" },
			{ value: "queue", label: "Queue" },
		],
	},
	{
		id: "runtime",
		label: "Runtime",
		tabs: [
			{ value: "connections", label: "Connections" },
			{ value: "logs", label: "Logs" },
			{ value: "metadata", label: "Metadata" },
		],
	},
] as const;

type TabGroupId = (typeof TAB_GROUPS)[number]["id"];

function groupForTab(value: string): TabGroupId {
	for (const group of TAB_GROUPS) {
		if (group.tabs.some((t) => t.value === value)) return group.id;
	}
	return "source";
}

const CODE_THEME_SETTINGS = {
	background: "transparent",
	lineHighlight: "transparent",
	gutterBackground: "transparent",
	gutterBorder: "hsl(var(--border))",
	fontSize: "12px",
} as const;

function ActorDetailPlaceholder({
	actorId,
	onClose,
}: { actorId: string; onClose: () => void }) {
	const [inspectorTab, setInspectorTab] = useState("code");
	const [theme] = useTheme();
	const codeTheme = useMemo(
		() =>
			theme === "dark"
				? githubDarkInit({ settings: CODE_THEME_SETTINGS })
				: githubLightInit({ settings: CODE_THEME_SETTINGS }),
		[theme],
	);
	const activeGroupId = groupForTab(inspectorTab);
	const activeGroup = TAB_GROUPS.find((g) => g.id === activeGroupId)!;

	const handleGroupChange = useCallback((groupId: string) => {
		const group = TAB_GROUPS.find((g) => g.id === groupId);
		if (group) setInspectorTab(group.tabs[0].value);
	}, []);

	return (
		<Tabs
			value={inspectorTab}
			onValueChange={setInspectorTab}
			className="flex flex-col flex-1 min-h-0"
		>
			<div className="flex items-center border-b h-[37px] px-1">
				<div className="flex-1 flex items-center gap-0.5">
					{TAB_GROUPS.map((group) => (
						<Button
							key={group.id}
							variant="ghost"
							size="sm"
							onClick={() => handleGroupChange(group.id)}
							className={cn(
								"h-7 text-xs px-2.5 rounded-md",
								group.id === activeGroupId
									? "bg-accent text-foreground"
									: "text-muted-foreground",
							)}
						>
							{group.label}
						</Button>
					))}
				</div>
				<Button variant="ghost" size="icon-sm" onClick={onClose}>
					<Icon icon={faXmark} />
				</Button>
			</div>
			<div className="flex items-center border-b h-[34px] px-1 bg-muted/30">
				<TabsList className="border-none h-full items-center bg-transparent gap-0.5">
					{activeGroup.tabs.map((tab) => (
						<TabsTrigger
							key={tab.value}
							value={tab.value}
							className="text-xs px-2.5 py-1 h-7 min-h-0 rounded-md font-medium border-b-0 data-[state=active]:border-b-transparent data-[state=active]:bg-accent"
						>
							{tab.label}
						</TabsTrigger>
					))}
				</TabsList>
			</div>
			<TabsContent value="code" className="flex-1 mt-0" forceMount>
				<div
					className={cn(
						"h-full p-3",
						inspectorTab !== "code" && "hidden",
					)}
				>
					<CodeMirror
						value={EXAMPLE_CODE}
						theme={codeTheme}
						extensions={[javascript({ typescript: true })]}
						readOnly
						basicSetup={{
							lineNumbers: true,
							foldGutter: false,
						}}
						className="h-full text-xs"
					/>
				</div>
			</TabsContent>
			<TabsContent
				value={inspectorTab}
				className="flex-1 mt-0"
				forceMount
			>
				{inspectorTab !== "code" ? (
					<ScrollArea className="h-full">
						<div className="p-4">
							<p className="text-muted-foreground text-sm text-center py-8">
								Inspector content will render here.
							</p>
						</div>
					</ScrollArea>
				) : null}
			</TabsContent>
		</Tabs>
	);
}

// -- Main Layout --

export function LayoutMockup() {
	const [selectedActorId, setSelectedActorId] = useState<string | null>(null);
	const [leftOpen, setLeftOpen] = useState(false);
	const [leftTab, setLeftTab] = useState<"agent" | "versions">("agent");

	// Track mount state for animations (start hidden, animate in)
	const [leftMounted, setLeftMounted] = useState(false);
	const [rightMounted, setRightMounted] = useState(false);

	const handleActorClick = useCallback((actorId: string) => {
		if (!actorId) {
			setSelectedActorId(null);
			return;
		}
		setSelectedActorId(actorId);
		setRightMounted(true);
	}, []);

	const handleLeftOpen = useCallback((tab: "agent" | "versions") => {
		setLeftTab(tab);
		setLeftOpen(true);
		setLeftMounted(true);
	}, []);

	const handleLeftClose = useCallback(() => {
		setLeftOpen(false);
	}, []);

	const handleRightClose = useCallback(() => {
		setSelectedActorId(null);
	}, []);

	return (
		<div className="fixed inset-0 flex flex-col bg-background dark:bg-black z-50">
			<MockupTopBar />

			<div className="flex-1 relative min-h-0 p-2">
				{/* Canvas fills entire area with inset and rounded corners */}
				<div className="absolute inset-2 rounded-lg overflow-hidden border dark:border-white/10 bg-card">
					<ActorCanvas
						onActorClick={handleActorClick}
						leftOpen={leftOpen}
						rightOpen={!!selectedActorId}
					/>
				</div>

				{/* Left floating buttons (visible when left popover is closed) */}
				<LeftFloatingButtons
					onOpen={handleLeftOpen}
					visible={!leftOpen}
				/>

				{/* Right floating buttons (visible when right popover is closed) */}
				<RightFloatingButtons visible={!selectedActorId} />

				{/* Left popover (always mounted after first open for animation) */}
				{leftMounted ? (
					<LeftPopover
						tab={leftTab}
						onTabChange={setLeftTab}
						onClose={handleLeftClose}
						visible={leftOpen}
					/>
				) : null}

				{/* Right popover (always mounted after first actor click for animation) */}
				{rightMounted ? (
					<RightPopover
						actorId={selectedActorId ?? "placeholder-0"}
						onClose={handleRightClose}
						onSelectActor={handleActorClick}
						visible={!!selectedActorId}
					/>
				) : null}
			</div>
		</div>
	);
}
