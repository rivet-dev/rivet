import {
	faActorsBorderless,
	faMagnifyingGlass,
	faPlus,
	faSliders,
	faXmark,
	Icon,
} from "@rivet-gg/icons";
import {
	Background,
	BackgroundVariant,
	Controls,
	type Edge,
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
import { useInfiniteQuery } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
	Button,
	cn,
	ScrollArea,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@/components";
import { CodeMirror } from "@/components/code-mirror";
import { ensureTrailingSlash } from "@/lib/utils";
import { ActorStatusIndicator } from "../actor-status-indicator";
import { useDataProvider } from "../data-provider";
import type { ActorId } from "../queries";

// -- Top Bar --

function MockupTopBar() {
	return (
		<div className="h-12 border-b bg-background flex items-center justify-between px-3 shrink-0 z-20">
			<div className="flex items-center gap-3">
				<img
					src={`${ensureTrailingSlash(import.meta.env.BASE_URL || "")}logo.svg`}
					alt="Rivet"
					className="h-5"
				/>
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
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground"
				>
					Billing
				</Button>
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground"
				>
					Support
				</Button>
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground"
				>
					What's New
				</Button>
				<div className="h-5 w-px bg-border mx-1" />
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground font-mono text-xs"
				>
					Company &#9662;
				</Button>
			</div>
		</div>
	);
}

function NamespaceDropdownPlaceholder() {
	return (
		<Button
			variant="ghost"
			size="sm"
			className="text-sm text-foreground font-medium"
		>
			Namespace
			<span className="text-muted-foreground ml-1 text-xs">
				&#9662;
			</span>
		</Button>
	);
}

// -- Actor Node for Canvas --

interface ActorNodeData {
	actorId: ActorId;
	label: string;
	instances: number;
	[key: string]: unknown;
}

function ActorCanvasNode({ data }: NodeProps<Node<ActorNodeData>>) {
	return (
		<div className="bg-card border rounded-lg px-4 py-3 w-[220px] cursor-pointer hover:border-primary/50 transition-colors shadow-sm relative group">
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
					className="text-muted-foreground"
				/>
				<div className="min-w-0">
					<div className="text-sm truncate">{data.label}</div>
					<div className="text-[10px] text-muted-foreground">
						{data.instances} instance{data.instances !== 1 ? "s" : ""}
					</div>
				</div>
			</div>
		</div>
	);
}

const nodeTypes: NodeTypes = {
	actor: ActorCanvasNode,
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

const PLACEHOLDER_NAMES = [
	"Leaderboard",
	"Chat Room",
	"Game Lobby",
	"Auth Session",
	"Match Maker",
	"Inventory",
	"Player Stats",
	"World State",
];

const PLACEHOLDER_INSTANCES = [3, 12, 1, 48, 5, 7, 1, 2];

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

	return PLACEHOLDER_NAMES.map((name, i) => ({
		id: `placeholder-${i}`,
		type: "actor",
		position: {
			x: (i % COLS) * X_GAP,
			y: Math.floor(i / COLS) * Y_GAP,
		},
		data: {
			actorId: `placeholder-${i}` as ActorId,
			label: name,
			instances: PLACEHOLDER_INSTANCES[i],
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
		if (saved?.nodes?.length) return saved.nodes;
		return makeDefaultNodes();
	});

	const [edges, setEdges] = useState<Edge[]>(() => {
		const saved = loadCanvasState();
		return saved?.edges ?? [];
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
				},
			})),
		);
	}, [actors]);

	// Persist state on changes
	const saveTimeout = useRef<ReturnType<typeof setTimeout>>();
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
				deleteKeyCode="Backspace"
				defaultEdgeOptions={{
					style: { stroke: "hsl(var(--muted-foreground))", strokeWidth: 1.5 },
					type: "smoothstep",
					markerEnd: {
						type: MarkerType.ArrowClosed,
						color: "hsl(var(--muted-foreground))",
					},
				}}
			>
				<Background
					variant={BackgroundVariant.Dots}
					gap={20}
					size={1.5}
					color="hsl(var(--border))"
				/>
				<Controls />
			</ReactFlow>
		</ReactFlowProvider>
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
				"absolute left-4 top-4 bottom-4 w-80 bg-card border rounded-lg shadow-xl z-10 flex flex-col overflow-hidden transition-transform duration-300 ease-out",
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
				<div className="flex items-center justify-between h-[45px]">
					<TabsList className="flex-1 items-end h-full">
						<TabsTrigger
							value="agent"
							className="text-xs px-3 py-1 pb-2"
						>
							Agent
						</TabsTrigger>
						<TabsTrigger
							value="versions"
							className="text-xs px-3 py-1 pb-2"
						>
							Versions
						</TabsTrigger>
					</TabsList>
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onClose}
						className="mr-2"
					>
						<Icon icon={faXmark} />
					</Button>
				</div>
				<TabsContent value="agent" className="flex-1 mt-0">
					<AgentChat />
				</TabsContent>
				<TabsContent value="versions" className="flex-1 mt-0">
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
		<div className="flex flex-col h-full">
			<ScrollArea className="flex-1">
				<div className="p-3 space-y-3">
					{FAKE_MESSAGES.map((msg, i) => (
						<div
							key={i}
							className={cn(
								"rounded-lg px-3 py-2 text-xs leading-relaxed max-w-[90%]",
								msg.role === "user"
									? "bg-primary text-primary-foreground ml-auto"
									: "bg-accent text-accent-foreground",
							)}
						>
							{msg.text}
						</div>
					))}
				</div>
			</ScrollArea>
			<div className="border-t p-2">
				<div className="flex gap-2">
					<input
						type="text"
						placeholder="Ask the agent..."
						className="flex-1 bg-transparent border rounded-md px-3 py-1.5 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
					/>
					<Button size="sm" className="text-xs">
						Send
					</Button>
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
				"absolute left-4 top-4 flex gap-2 z-10 transition-all duration-300 ease-out",
				visible
					? "translate-x-0 opacity-100"
					: "-translate-x-8 opacity-0 pointer-events-none",
			)}
		>
			<Button
				variant="secondary"
				className="shadow-lg"
				onClick={() => onOpen("agent")}
			>
				Agent
			</Button>
			<Button
				variant="secondary"
				className="shadow-lg"
				onClick={() => onOpen("versions")}
			>
				Versions
			</Button>
		</div>
	);
}

// -- Right Popover (Actor List + Details) --

function RightPopover({
	actorId,
	onClose,
	visible,
}: {
	actorId: string;
	onClose: () => void;
	visible: boolean;
}) {
	return (
		<div
			className={cn(
				"absolute right-4 top-4 bottom-4 w-[40%] min-w-[500px] bg-card border rounded-lg shadow-xl z-10 flex flex-col overflow-hidden transition-transform duration-300 ease-out",
				visible
					? "translate-x-0"
					: "translate-x-[calc(100%+2rem)]",
			)}
		>
			<div className="flex flex-1 min-h-0">
				<div className="w-[220px] min-w-[220px] border-r flex flex-col">
					<ActorListToolbar />
					<ActorListPlaceholder selectedActorId={actorId} />
				</div>
				<div className="flex-1 flex flex-col min-w-0 relative">
					<ActorDetailPlaceholder actorId={actorId} />
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onClose}
						className="absolute top-1.5 right-2 z-10"
					>
						<Icon icon={faXmark} />
					</Button>
				</div>
			</div>
		</div>
	);
}

function ActorListToolbar() {
	return (
		<div className="flex items-center justify-end gap-1 border-b px-2 py-1.5">
			<Button variant="ghost" size="icon-sm">
				<Icon icon={faMagnifyingGlass} />
			</Button>
			<Button variant="ghost" size="icon-sm">
				<Icon icon={faPlus} />
			</Button>
			<Button variant="ghost" size="icon-sm">
				<Icon icon={faSliders} />
			</Button>
		</div>
	);
}

function ActorListPlaceholder({
	selectedActorId,
}: { selectedActorId: string }) {
	return (
		<ScrollArea className="flex-1">
			{Array.from({ length: 12 }, (_, i) => {
				const id = `placeholder-${i}`;
				const isSelected = id === selectedActorId;
				const statuses = [
					"running",
					"running",
					"sleeping",
					"running",
					"stopped",
					"running",
					"running",
					"sleeping",
					"running",
					"running",
					"crashed",
					"running",
				] as const;
				return (
					<div
						key={id}
						className={cn(
							"flex items-center gap-2 px-2.5 py-1.5 border-b text-sm cursor-pointer hover:bg-accent/50",
							isSelected && "bg-accent",
						)}
					>
						<ActorStatusIndicator status={statuses[i]} />
						<span className="font-mono text-xs truncate">
							actor-{i + 1}
						</span>
					</div>
				);
			})}
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

function ActorDetailPlaceholder({ actorId }: { actorId: string }) {
	const [inspectorTab, setInspectorTab] = useState("code");

	return (
		<Tabs
			value={inspectorTab}
			onValueChange={setInspectorTab}
			className="flex flex-col flex-1 min-h-0"
		>
			<div className="flex justify-between items-center border-b h-[45px]">
				<TabsList className="overflow-auto border-none h-full items-end [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
					<TabsTrigger
						value="code"
						className="text-xs px-3 py-1 pb-2"
					>
						Code
					</TabsTrigger>
					<TabsTrigger
						value="workflow"
						className="text-xs px-3 py-1 pb-2"
					>
						Workflow
					</TabsTrigger>
					<TabsTrigger
						value="database"
						className="text-xs px-3 py-1 pb-2"
					>
						Database
					</TabsTrigger>
					<TabsTrigger
						value="state"
						className="text-xs px-3 py-1 pb-2"
					>
						State
					</TabsTrigger>
					<TabsTrigger
						value="queue"
						className="text-xs px-3 py-1 pb-2"
					>
						Queue
					</TabsTrigger>
					<TabsTrigger
						value="connections"
						className="text-xs px-3 py-1 pb-2"
					>
						Connections
					</TabsTrigger>
					<TabsTrigger
						value="metadata"
						className="text-xs px-3 py-1 pb-2"
					>
						Metadata
					</TabsTrigger>
					<TabsTrigger
						value="logs"
						className="text-xs px-3 py-1 pb-2"
					>
						Logs
					</TabsTrigger>
				</TabsList>
			</div>
			<TabsContent value="code" className="flex-1 mt-0" forceMount>
				<div
					className={cn(
						"h-full overflow-auto",
						inspectorTab !== "code" && "hidden",
					)}
				>
					<CodeMirror
						value={EXAMPLE_CODE}
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
		<div className="fixed inset-0 flex flex-col bg-background z-50">
			<MockupTopBar />

			<div className="flex-1 relative min-h-0 p-2">
				{/* Canvas fills entire area with inset and rounded corners */}
				<div className="absolute inset-2 rounded-lg overflow-hidden border bg-card">
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
						visible={!!selectedActorId}
					/>
				) : null}
			</div>
		</div>
	);
}
