import * as DialogPrimitive from "@radix-ui/react-dialog";
import {
	faActorsBorderless,
	faArrowRight,
	faBolt,
	faBracketsCurly,
	faChevronDown,
	faChevronLeft,
	faChevronRight,
	faClockRotateLeft,
	faCode,
	faCodeBranch,
	faComment,
	faDatabase,
	faDiagramProject,
	faFileLines,
	faInbox,
	faMagnifyingGlass,
	faMicrochip,
	faPlug,
	faPlus,
	faSliders,
	faTag,
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
} from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	cn,
	Input,
	Popover,
	PopoverContent,
	PopoverTrigger,
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	ScrollArea,
	Switch,
} from "@/components";
import { CodeMirror } from "@/components/code-mirror";
import {
	MockupTopBar,
	useMockupTheme,
} from "@/components/mockup/shared";
import { ActorStatusIndicator } from "../actor-status-indicator";
import { useDataProvider } from "../data-provider";
import type { ActorId, ActorStatus } from "../queries";

// -- Actor Node for Canvas --

type ActorType = "realtime" | "workflow" | "sqlite" | "vm";

const ACTOR_TYPE_META: Record<
	ActorType,
	{ label: string; icon: typeof faBolt }
> = {
	realtime: { label: "Realtime", icon: faBolt },
	workflow: { label: "Workflow", icon: faDiagramProject },
	sqlite: { label: "SQLite", icon: faDatabase },
	vm: { label: "Virtual Machine", icon: faMicrochip },
};

interface ActorNodeData {
	actorId: ActorId;
	label: string;
	instances: number;
	status: ActorStatus;
	version: string;
	actorType: ActorType;
	[key: string]: unknown;
}

function ActorCanvasNode({ data, selected }: NodeProps<Node<ActorNodeData>>) {
	const typeMeta =
		ACTOR_TYPE_META[data.actorType] ?? ACTOR_TYPE_META.realtime;

	return (
		<div
			className={cn(
				"bg-card border rounded-lg px-3 py-2.5 w-[240px] cursor-pointer hover:border-foreground/40 hover:shadow-md transition-all shadow-sm relative group",
				selected && "border-foreground ring-2 ring-foreground/30",
			)}
		>
			<Handle
				type="source"
				position={Position.Top}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-20 group-hover:opacity-100 transition-opacity"
			/>
			<Handle
				type="source"
				position={Position.Right}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-20 group-hover:opacity-100 transition-opacity"
				id="right"
			/>
			<Handle
				type="target"
				position={Position.Bottom}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-20 group-hover:opacity-100 transition-opacity"
				id="bottom"
			/>
			<Handle
				type="target"
				position={Position.Left}
				className="!w-2 !h-2 !bg-muted-foreground/50 !border-none opacity-20 group-hover:opacity-100 transition-opacity"
				id="left"
			/>
			<div className="flex items-center gap-2.5">
				<div className="flex items-center justify-center w-8 h-8 rounded-md border bg-muted/40 text-muted-foreground shrink-0">
					<Icon icon={typeMeta.icon} className="text-xs" />
				</div>
				<div className="min-w-0 flex-1">
					<div className="text-sm truncate">{data.label}</div>
					<div className="text-[11px] text-muted-foreground truncate">
						{data.instances} instance
						{data.instances !== 1 ? "s" : ""}
						<span className="opacity-40 mx-1">·</span>
						<span className="font-mono">{data.version}</span>
					</div>
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

	const selectedMarkerId = `arrow-selected-${id}`;

	return (
		<>
			{selected ? (
				<defs>
					<marker
						id={selectedMarkerId}
						viewBox="-10 -10 20 20"
						refX="0"
						refY="0"
						markerWidth="12.5"
						markerHeight="12.5"
						markerUnits="strokeWidth"
						orient="auto-start-reverse"
					>
						<polyline
							style={{
								stroke: "hsl(var(--foreground))",
								fill: "hsl(var(--foreground))",
								strokeWidth: 1,
							}}
							points="-5,-4 0,0 -5,4 -5,-4"
						/>
					</marker>
				</defs>
			) : null}
			<BaseEdge
				path={edgePath}
				style={{
					...style,
					stroke: selected
						? "hsl(var(--foreground))"
						: style?.stroke,
					strokeWidth: selected ? 2 : style?.strokeWidth,
				}}
				markerEnd={
					selected ? `url(#${selectedMarkerId})` : markerEnd
				}
			/>
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
							className="flex items-center justify-center w-5 h-5 rounded-full bg-background border border-foreground text-foreground hover:bg-destructive hover:text-destructive-foreground hover:border-destructive shadow-sm transition-colors"
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

const STORAGE_KEY = "mockup-canvas-state-v2";

interface ActorInstance {
	key: string;
	status: ActorStatus;
	region: string;
	uptime: string;
}

const INSTANCE_REGIONS = ["us-east", "us-west", "eu-west", "ap-south"];

function hashSeed(n: number): number {
	let x = n * 2654435761;
	x ^= x >>> 16;
	x = Math.imul(x, 2246822507);
	x ^= x >>> 13;
	x = Math.imul(x, 3266489909);
	x ^= x >>> 16;
	return x >>> 0;
}

function toShortKey(seed: number): string {
	return seed.toString(36).padStart(8, "0").slice(-8);
}

function formatUptime(hours: number): string {
	if (hours < 1) return `${Math.max(1, Math.floor(hours * 60))}m`;
	if (hours < 24) return `${Math.floor(hours)}h`;
	const days = Math.floor(hours / 24);
	if (days < 30) return `${days}d`;
	return `${Math.floor(days / 7)}w`;
}

function generateInstances(
	actorIndex: number,
	count: number,
	baseStatus: ActorStatus,
): ActorInstance[] {
	const instances: ActorInstance[] = [];
	for (let i = 0; i < count; i++) {
		const seed = hashSeed(actorIndex * 1_000_003 + i * 7919 + 1);
		let status: ActorStatus = baseStatus;
		if (baseStatus === "running") {
			if (seed % 19 === 0) status = "sleeping";
			else if (seed % 23 === 0) status = "starting";
		}
		const region = INSTANCE_REGIONS[seed % INSTANCE_REGIONS.length];
		const hoursAlive = ((seed >> 8) % 720) + 1;
		instances.push({
			key: toShortKey(seed),
			status,
			region,
			uptime: formatUptime(hoursAlive),
		});
	}
	return instances;
}

const PLACEHOLDERS: {
	name: string;
	instances: number;
	status: ActorStatus;
	version: string;
	actorType: ActorType;
}[] = [
	{ name: "Leaderboard", instances: 3, status: "running", version: "v12", actorType: "sqlite" },
	{ name: "Chat Room", instances: 12, status: "running", version: "v8", actorType: "realtime" },
	{ name: "Game Lobby", instances: 1, status: "sleeping", version: "v4", actorType: "realtime" },
	{ name: "Auth Session", instances: 48, status: "running", version: "v21", actorType: "realtime" },
	{ name: "Match Maker", instances: 5, status: "starting", version: "v2", actorType: "workflow" },
	{ name: "Inventory", instances: 7, status: "running", version: "v15", actorType: "sqlite" },
	{ name: "Player Stats", instances: 1, status: "crashed", version: "v3", actorType: "sqlite" },
	{ name: "World State", instances: 2, status: "running", version: "v9", actorType: "realtime" },
	{ name: "Chat Presence", instances: 6, status: "running", version: "v7", actorType: "realtime" },
	{ name: "Payments Worker", instances: 2, status: "running", version: "v18", actorType: "workflow" },
	{ name: "Email Queue", instances: 1, status: "sleeping", version: "v5", actorType: "workflow" },
	{ name: "Analytics Sink", instances: 4, status: "running", version: "v11", actorType: "vm" },
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
			actorType: placeholder.actorType,
		},
	}));
}

function makeDefaultEdges(): Edge[] {
	const connections: [number, number][] = [
		[3, 1],
		[3, 0],
		[1, 8],
		[4, 2],
		[4, 7],
		[5, 6],
		[5, 11],
		[9, 10],
		[0, 7],
	];
	return connections.map(([from, to], i) => ({
		id: `placeholder-edge-${i}`,
		source: `placeholder-${from}`,
		target: `placeholder-${to}`,
		type: "deletable",
	}));
}

// -- Actor Canvas --

function ActorCanvas({
	onActorClick,
	leftOpen,
	rightOpen,
	selectedActorId,
	onCreate,
}: {
	onActorClick: (actorId: string) => void;
	leftOpen: boolean;
	rightOpen: boolean;
	selectedActorId: string | null;
	onCreate: () => void;
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
		if (saved?.edges?.length) {
			return saved.edges.map((edge) => ({
				...edge,
				type: "deletable",
			}));
		}
		return makeDefaultEdges();
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
					actorType: "realtime",
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

	const displayedNodes = useMemo(
		() =>
			nodes.map((node) => ({
				...node,
				selected: selectedActorId !== null && node.id === selectedActorId,
			})),
		[nodes, selectedActorId],
	);

	const displayedEdges = useMemo(
		() =>
			edges.map((edge) => {
				const connected =
					selectedActorId === null ||
					edge.source === selectedActorId ||
					edge.target === selectedActorId;
				return {
					...edge,
					className: cn(
						"transition-opacity duration-200",
						!connected && "opacity-25",
					),
				};
			}),
		[edges, selectedActorId],
	);

	return (
		<ReactFlowProvider>
			<FitViewOnChange leftOpen={leftOpen} rightOpen={rightOpen} />
			<ReactFlow
				nodes={displayedNodes}
				edges={displayedEdges}
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
					style={{
						maskImage:
							"radial-gradient(ellipse 75% 75% at 50% 50%, black 45%, transparent 95%)",
						WebkitMaskImage:
							"radial-gradient(ellipse 75% 75% at 50% 50%, black 45%, transparent 95%)",
					}}
				/>
				<Controls
					showInteractive={false}
					className="!shadow-sm !border !border-border !rounded-md overflow-hidden [&>button]:!bg-card [&>button]:!border-b [&>button]:!border-border [&>button]:!text-foreground hover:[&>button]:!bg-accent"
				/>
			</ReactFlow>
			{nodes.length === 0 ? <CanvasEmptyState onCreate={onCreate} /> : null}
		</ReactFlowProvider>
	);
}

function CanvasEmptyState({ onCreate }: { onCreate: () => void }) {
	return (
		<div className="absolute inset-0 flex items-center justify-center pointer-events-none z-[5]">
			<div className="flex flex-col items-center gap-4 pointer-events-auto">
				<div className="w-16 h-16 rounded-lg border-2 border-dashed border-muted-foreground/30 flex items-center justify-center">
					<Icon
						icon={faActorsBorderless}
						className="text-2xl text-muted-foreground/60"
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
				<Button
					size="sm"
					className="gap-1.5 text-xs px-3 bg-foreground text-background hover:bg-foreground/90"
					onClick={onCreate}
				>
					<Icon icon={faPlus} className="w-3" />
					Create actor
				</Button>
			</div>
		</div>
	);
}

// -- Create Actor Dialog --

interface CreateOption {
	id: string;
	name: string;
	description: string;
	icon: typeof faBolt;
}

const CREATE_OPTION_GROUPS: {
	group: string;
	options: CreateOption[];
}[] = [
	{
		group: "Actor",
		options: [
			{
				id: "realtime",
				name: "Realtime",
				description:
					"Stateful actors for multiplayer, chat, and live collaboration.",
				icon: faBolt,
			},
			{
				id: "workflow",
				name: "Workflow",
				description:
					"Durable, long-running processes with retries and versioning.",
				icon: faDiagramProject,
			},
			{
				id: "sqlite",
				name: "SQLite",
				description:
					"Per-actor embedded SQLite database with strong consistency.",
				icon: faDatabase,
			},
		],
	},
	{
		group: "Agent OS",
		options: [
			{
				id: "virtual-machine",
				name: "Virtual Machine",
				description:
					"Isolated VM for running untrusted code and agentic workloads.",
				icon: faMicrochip,
			},
		],
	},
];

function MockSelect({
	value,
	leading,
}: {
	value: string;
	leading?: React.ReactNode;
}) {
	return (
		<button
			type="button"
			className="flex h-9 w-full items-center justify-between gap-2 rounded-md border dark:border-white/10 bg-background px-3 text-sm text-foreground hover:bg-accent/40 transition-colors focus-visible:outline-none focus-visible:border-foreground/40"
		>
			<span className="flex items-center gap-2 min-w-0 truncate">
				{leading}
				<span className="truncate">{value}</span>
			</span>
			<Icon
				icon={faChevronDown}
				className="w-3 text-muted-foreground shrink-0"
			/>
		</button>
	);
}

function FieldGroup({
	label,
	description,
	children,
}: {
	label: string;
	description: string;
	children: React.ReactNode;
}) {
	return (
		<div className="space-y-1.5">
			<label className="text-sm font-medium text-foreground">
				{label}
			</label>
			{children}
			<p className="text-xs text-muted-foreground">{description}</p>
		</div>
	);
}

function CreateActorTemplateStep({
	onSelect,
}: {
	onSelect: (option: CreateOption) => void;
}) {
	return (
		<>
			<div className="flex items-start justify-between gap-4 px-5 pt-5 pb-2">
				<div>
					<DialogPrimitive.Title className="text-base font-semibold text-foreground">
						Create actor
					</DialogPrimitive.Title>
					<DialogPrimitive.Description className="mt-0.5 text-xs text-muted-foreground">
						Choose a template to get started.
					</DialogPrimitive.Description>
				</div>
				<DialogPrimitive.Close
					className="rounded-sm text-muted-foreground opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					aria-label="Close"
				>
					<Icon icon={faXmark} className="h-4 w-4" />
				</DialogPrimitive.Close>
			</div>
			<div className="px-5 pt-2 pb-5 max-h-[70vh] overflow-auto">
				{CREATE_OPTION_GROUPS.map((group) => (
					<div key={group.group} className="mt-3 first:mt-0">
						<div className="pb-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
							{group.group}
						</div>
						<div className="flex flex-col gap-2">
							{group.options.map((option) => (
								<button
									key={option.id}
									type="button"
									onClick={() => onSelect(option)}
									className="flex items-center gap-3 px-3 py-2.5 rounded-md border dark:border-white/10 text-left hover:border-foreground/30 hover:bg-accent/40 transition-colors"
								>
									<div className="flex items-center justify-center w-8 h-8 rounded-md shrink-0 bg-muted text-muted-foreground">
										<Icon
											icon={option.icon}
											className="text-sm"
										/>
									</div>
									<div className="flex-1 min-w-0">
										<div className="text-sm font-medium text-foreground">
											{option.name}
										</div>
										<div className="text-[11px] text-muted-foreground leading-relaxed">
											{option.description}
										</div>
									</div>
								</button>
							))}
						</div>
					</div>
				))}
			</div>
		</>
	);
}

function CreateActorConfigureStep({
	option,
	onBack,
	onCreate,
}: {
	option: CreateOption;
	onBack: () => void;
	onCreate: () => void;
}) {
	const actorName = `${option.id.replace(/-/g, "")}Data`;
	const [key, setKey] = useState("thin-socket");

	return (
		<>
			<div className="flex items-start justify-between gap-4 px-5 pt-5 pb-2">
				<div className="flex items-start gap-3 min-w-0">
					<button
						type="button"
						onClick={onBack}
						className="mt-0.5 rounded-sm p-1 -ml-1 text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						aria-label="Back"
					>
						<Icon icon={faChevronLeft} className="w-3.5" />
					</button>
					<div className="min-w-0">
						<DialogPrimitive.Title className="text-base font-semibold text-foreground truncate">
							Create '{actorName}'
						</DialogPrimitive.Title>
						<DialogPrimitive.Description className="mt-0.5 text-xs text-muted-foreground">
							Provide the necessary details to create an actor.
						</DialogPrimitive.Description>
					</div>
				</div>
				<DialogPrimitive.Close
					className="rounded-sm text-muted-foreground opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					aria-label="Close"
				>
					<Icon icon={faXmark} className="h-4 w-4" />
				</DialogPrimitive.Close>
			</div>
			<div className="max-h-[70vh] overflow-auto">
				<div className="px-5 pt-5 pb-4 space-y-5">
					<FieldGroup
						label="Key"
						description="Identifier for the Actor."
					>
						<Input
							value={key}
							onChange={(e) => setKey(e.target.value)}
							className="font-mono text-xs h-9"
						/>
					</FieldGroup>
					<FieldGroup
						label="Code"
						description="You can use the snippet above to get or create the actor in your application."
					>
						<div className="rounded-md border dark:border-white/10 bg-muted/30 px-3 py-2 font-mono text-xs text-foreground overflow-x-auto">
							<span className="text-muted-foreground">client.</span>
							<span className="text-amber-500">{actorName}</span>
							<span className="text-muted-foreground">
								.getOrCreate(
							</span>
							<span className="text-emerald-500">"{key}"</span>
							<span className="text-muted-foreground">)</span>
						</div>
					</FieldGroup>
				</div>
				<Accordion
					type="single"
					collapsible
					defaultValue="advanced"
					className="border-t dark:border-white/10"
				>
					<AccordionItem
						value="advanced"
						className="border-b-0"
					>
						<AccordionTrigger className="px-5 py-3 text-sm font-semibold text-foreground hover:no-underline">
							Advanced
						</AccordionTrigger>
						<AccordionContent className="px-5 pt-1 pb-5 space-y-5">
							<FieldGroup
								label="Datacenter"
								description="The datacenter where the Actor will be deployed."
							>
								<MockSelect
									value="Northern Virginia, USA"
									leading={
										<span className="text-base leading-none">
											🇺🇸
										</span>
									}
								/>
							</FieldGroup>
							<FieldGroup
								label="Runner"
								description="Runner name selector for the actor. Used to select which runner the actor will run on."
							>
								<MockSelect value="default" />
							</FieldGroup>
							<FieldGroup
								label="Crash Policy"
								description="Determines the behavior of the actor on crash."
							>
								<MockSelect value="Sleep" />
							</FieldGroup>
							<FieldGroup
								label="Input"
								description="Optional JSON object that will be passed to the Actor as input."
							>
								<div className="rounded-md border dark:border-white/10 bg-muted/30 overflow-hidden">
									<div className="flex text-xs font-mono">
										<div className="select-none bg-muted/40 border-r dark:border-white/10 px-2 py-1.5 text-muted-foreground text-right w-8">
											1
										</div>
										<div className="flex-1 px-2 py-1.5 min-h-[64px] text-foreground">
											{" "}
										</div>
									</div>
								</div>
							</FieldGroup>
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</div>
			<div className="flex items-center justify-end gap-2 px-5 py-4 bg-muted/20">
				<Button
					variant="ghost"
					size="sm"
					onClick={onBack}
				>
					Back
				</Button>
				<Button
					size="sm"
					className="px-3 bg-foreground text-background hover:bg-foreground/90"
					onClick={onCreate}
				>
					Create
				</Button>
			</div>
		</>
	);
}

function CreateActorDialog({
	open,
	onOpenChange,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
}) {
	const [selected, setSelected] = useState<CreateOption | null>(null);

	useEffect(() => {
		if (!open) {
			const t = setTimeout(() => setSelected(null), 150);
			return () => clearTimeout(t);
		}
	}, [open]);

	return (
		<DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
			<DialogPrimitive.Portal>
				<DialogPrimitive.Overlay
					className={cn(
						"fixed inset-0 z-50 bg-black/50 grid place-items-center",
						"data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:duration-150",
						"data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-100",
					)}
				>
					<DialogPrimitive.Content
						className={cn(
							"relative z-50 w-[92vw] max-w-lg",
							"rounded-lg border dark:border-white/10 bg-card shadow-2xl overflow-hidden",
							"data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=open]:duration-150",
							"data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:duration-100",
						)}
					>
						{selected ? (
							<CreateActorConfigureStep
								option={selected}
								onBack={() => setSelected(null)}
								onCreate={() => onOpenChange(false)}
							/>
						) : (
							<CreateActorTemplateStep onSelect={setSelected} />
						)}
					</DialogPrimitive.Content>
				</DialogPrimitive.Overlay>
			</DialogPrimitive.Portal>
		</DialogPrimitive.Root>
	);
}

// -- Left Popover (Agent / Versions) --

function LeftPopover({
	tab,
	onClose,
	visible,
}: {
	tab: "agent" | "versions";
	onClose: () => void;
	visible: boolean;
}) {
	const [agentView, setAgentView] = useState<"chat" | "history">("chat");
	const [activeThreadId, setActiveThreadId] = useState<string | null>(null);

	return (
		<div
			className={cn(
				"absolute left-4 top-14 bottom-4 w-80 bg-card border dark:border-white/10 rounded-lg shadow-lg z-10 flex flex-col overflow-hidden transition-all duration-200 ease-out",
				visible
					? "translate-x-0 opacity-100"
					: "-translate-x-[calc(100%+2rem)] opacity-0 pointer-events-none",
			)}
		>
			<div className="flex items-center justify-between border-b h-[37px] pl-3 pr-2 shrink-0">
				<span className="text-xs font-medium text-foreground">
					{tab === "agent"
						? agentView === "history"
							? "Chat history"
							: "Agent"
						: "Versions"}
				</span>
				<div className="flex items-center gap-0.5">
					{tab === "agent" ? (
						<>
							<Button
								variant="ghost"
								size="icon-sm"
								onClick={() => {
									setActiveThreadId(null);
									setAgentView("chat");
								}}
								aria-label="New chat"
							>
								<Icon icon={faPlus} />
							</Button>
							<Button
								variant="ghost"
								size="icon-sm"
								onClick={() =>
									setAgentView((v) =>
										v === "chat" ? "history" : "chat",
									)
								}
								aria-label={
									agentView === "history"
										? "Back to chat"
										: "Chat history"
								}
							>
								<Icon
									icon={
										agentView === "history"
											? faComment
											: faClockRotateLeft
									}
								/>
							</Button>
						</>
					) : null}
					<Button
						variant="ghost"
						size="icon-sm"
						onClick={onClose}
						aria-label="Close panel"
					>
						<Icon icon={faXmark} />
					</Button>
				</div>
			</div>
			<div className="flex-1 min-h-0">
				{tab === "agent" ? (
					agentView === "history" ? (
						<AgentHistoryList
							activeId={activeThreadId}
							onSelect={(id) => {
								setActiveThreadId(id);
								setAgentView("chat");
							}}
						/>
					) : (
						<AgentChat threadId={activeThreadId} />
					)
				) : (
					<VersionsList />
				)}
			</div>
		</div>
	);
}

// -- Versions List --

interface MockVersion {
	id: string;
	name: string;
	deployedAt: string;
}

const MOCK_VERSIONS: MockVersion[] = [
	{ id: "v-12", name: "v12 · feat/leaderboard", deployedAt: "2h ago" },
	{ id: "v-11", name: "v11 · fix/reconnect", deployedAt: "1d ago" },
	{ id: "v-10", name: "v10 · refactor/state", deployedAt: "3d ago" },
	{ id: "v-9", name: "v9 · chore/deps", deployedAt: "1w ago" },
	{ id: "v-8", name: "v8 · feat/presence", deployedAt: "2w ago" },
];

function VersionsList() {
	const [currentId, setCurrentId] = useState(MOCK_VERSIONS[0].id);
	return (
		<ScrollArea className="h-full">
			{MOCK_VERSIONS.map((version) => {
				const isCurrent = version.id === currentId;
				return (
					<div
						key={version.id}
						className="flex items-center gap-2 pl-3 pr-2 py-2 border-b dark:border-white/10 last:border-b-0 hover:bg-accent/40 transition-colors"
					>
						<div className="flex-1 min-w-0">
							<div className="text-xs text-foreground truncate">
								{version.name}
							</div>
							<div className="text-[11px] text-muted-foreground mt-0.5">
								Deployed {version.deployedAt}
							</div>
						</div>
						{isCurrent ? (
							<span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground px-2">
								Current
							</span>
						) : (
							<Button
								variant="outline"
								size="sm"
								className="h-7 px-2.5 text-[11px]"
								onClick={() => setCurrentId(version.id)}
							>
								Deploy
							</Button>
						)}
					</div>
				);
			})}
		</ScrollArea>
	);
}

// -- Fake Agent Chat UI --

interface AgentMessage {
	role: "user" | "assistant";
	text: string;
}

const EXAMPLE_PROMPTS: string[] = [
	"Add a real-time chat actor",
	"Build a leaderboard with SQLite",
	"Scaffold a payment workflow",
	"How do I handle actor crashes?",
	"Realtime vs Workflow — what's different?",
];

function AgentWelcome({ onPrompt }: { onPrompt: (prompt: string) => void }) {
	return (
		<div className="flex flex-col h-full px-4 pt-8 pb-4">
			<div className="mb-7 space-y-2">
				<div className="flex items-center gap-2">
					<span className="relative size-4 shrink-0 inline-block animate-dash-rotate">
						<svg
							className="absolute inset-0 h-full w-full text-muted-foreground/60"
							viewBox="0 0 16 16"
							fill="none"
							aria-hidden
						>
							<circle
								cx="8"
								cy="8"
								r="6"
								pathLength="24"
								stroke="currentColor"
								strokeWidth="1.2"
								strokeDasharray="0.9 1.1"
								strokeLinecap="round"
							/>
						</svg>
						<svg
							className="absolute inset-0 h-full w-full text-white animate-dash-chase"
							viewBox="0 0 16 16"
							fill="none"
							aria-hidden
						>
							<circle
								cx="8"
								cy="8"
								r="6"
								pathLength="24"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeDasharray="0.9 23.1"
								strokeLinecap="round"
							/>
						</svg>
					</span>
					<h2 className="text-base font-medium text-foreground tracking-tight">
						How can I help?
					</h2>
				</div>
				<p className="text-xs text-muted-foreground">
					Ask about Rivet Actors or describe what to build.
				</p>
			</div>
			<div className="flex flex-col">
				<div className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70 mb-2">
					Suggestions
				</div>
				<div className="flex flex-wrap gap-1.5">
					{EXAMPLE_PROMPTS.map((text) => (
						<button
							key={text}
							type="button"
							onClick={() => onPrompt(text)}
							className="inline-flex items-center rounded-full border border-input bg-background/80 px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
						>
							{text}
						</button>
					))}
				</div>
			</div>
		</div>
	);
}

interface MockChatThread {
	id: string;
	title: string;
	preview: string;
	updatedAt: string;
	messages: AgentMessage[];
}

const MOCK_CHAT_THREADS: MockChatThread[] = [
	{
		id: "thread-1",
		title: "Leaderboard with SQLite",
		preview: "Let's scaffold an actor with a sorted ranking table…",
		updatedAt: "2h ago",
		messages: [
			{
				role: "user",
				text: "Build a leaderboard with SQLite",
			},
			{
				role: "assistant",
				text: "Let's scaffold an actor with a sorted ranking table and a submitScore action.",
			},
		],
	},
	{
		id: "thread-2",
		title: "Real-time chat actor",
		preview: "Here's a Room actor pattern with presence and broadcast…",
		updatedAt: "Yesterday",
		messages: [
			{
				role: "user",
				text: "Add a real-time chat actor",
			},
			{
				role: "assistant",
				text: "Here's a Room actor pattern with presence and broadcast over WebSockets.",
			},
		],
	},
	{
		id: "thread-3",
		title: "Handling actor crashes",
		preview: "Crash loops fall back to the previous version, and you can…",
		updatedAt: "Mon",
		messages: [
			{
				role: "user",
				text: "How do I handle actor crashes?",
			},
			{
				role: "assistant",
				text: "Crash loops fall back to the previous version, and you can watch the Logs tab to debug.",
			},
		],
	},
	{
		id: "thread-4",
		title: "Payment workflow scaffold",
		preview: "Workflows are ideal for multi-step, durable operations…",
		updatedAt: "Last week",
		messages: [
			{
				role: "user",
				text: "Scaffold a payment workflow",
			},
			{
				role: "assistant",
				text: "Workflows are ideal for multi-step, durable operations. Here's a skeleton.",
			},
		],
	},
	{
		id: "thread-5",
		title: "Realtime vs Workflow",
		preview: "Realtime actors hold a persistent connection; workflows…",
		updatedAt: "Mar 14",
		messages: [
			{
				role: "user",
				text: "Realtime vs Workflow — what's different?",
			},
			{
				role: "assistant",
				text: "Realtime actors hold a persistent connection; workflows orchestrate long-running steps.",
			},
		],
	},
];

function AgentHistoryList({
	activeId,
	onSelect,
}: {
	activeId: string | null;
	onSelect: (id: string) => void;
}) {
	return (
		<ScrollArea className="h-full">
			{MOCK_CHAT_THREADS.map((thread) => {
				const isActive = thread.id === activeId;
				return (
					<button
						key={thread.id}
						type="button"
						onClick={() => onSelect(thread.id)}
						className={cn(
							"flex w-full flex-col gap-0.5 text-left px-3 py-2.5 border-b dark:border-white/10 last:border-b-0 transition-colors",
							isActive ? "bg-accent" : "hover:bg-accent/40",
						)}
					>
						<div className="flex items-center justify-between gap-2 min-w-0">
							<span className="text-xs font-medium text-foreground truncate">
								{thread.title}
							</span>
							<span className="text-[10px] text-muted-foreground shrink-0">
								{thread.updatedAt}
							</span>
						</div>
						<span className="text-[11px] text-muted-foreground truncate">
							{thread.preview}
						</span>
					</button>
				);
			})}
		</ScrollArea>
	);
}

function AgentChat({ threadId }: { threadId: string | null }) {
	const initialMessages =
		threadId != null
			? (MOCK_CHAT_THREADS.find((t) => t.id === threadId)?.messages ?? [])
			: [];
	const [messages, setMessages] = useState<AgentMessage[]>(initialMessages);
	const [input, setInput] = useState("");

	useEffect(() => {
		setMessages(
			threadId != null
				? (MOCK_CHAT_THREADS.find((t) => t.id === threadId)?.messages ??
						[])
				: [],
		);
	}, [threadId]);

	const send = useCallback((text: string) => {
		const trimmed = text.trim();
		if (!trimmed) return;
		setMessages((prev) => [...prev, { role: "user", text: trimmed }]);
		setInput("");
	}, []);

	return (
		<div className="flex flex-col h-full min-h-0">
			<ScrollArea className="flex-1">
				{messages.length === 0 ? (
					<AgentWelcome onPrompt={(p) => setInput(p)} />
				) : (
					<div className="px-4 py-4 space-y-5">
						{messages.map((msg, i) =>
							msg.role === "user" ? (
								<div key={i} className="flex justify-end">
									<div className="rounded-lg rounded-br-sm bg-accent text-foreground text-xs leading-relaxed px-3 py-2 max-w-[85%]">
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
				)}
			</ScrollArea>
			<div className="p-3">
				<form
					onSubmit={(e) => {
						e.preventDefault();
						send(input);
					}}
					className="flex items-center gap-2 rounded-lg border bg-muted/20 focus-within:bg-muted/40 focus-within:border-foreground/30 transition-colors px-2.5 py-1"
				>
					<input
						type="text"
						value={input}
						onChange={(e) => setInput(e.target.value)}
						placeholder="Ask the agent..."
						className="flex-1 min-w-0 bg-transparent text-xs placeholder:text-muted-foreground focus:outline-none py-1.5"
					/>
					<button
						type="submit"
						aria-label="Send message"
						className="w-6 h-6 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent flex items-center justify-center transition-colors shrink-0"
					>
						<Icon icon={faArrowRight} className="w-2.5" />
					</button>
				</form>
			</div>
		</div>
	);
}

// -- Left Floating Buttons --

function LeftFloatingButtons({
	onSelect,
	activeTab,
	isOpen,
}: {
	onSelect: (tab: "agent" | "versions") => void;
	activeTab: "agent" | "versions";
	isOpen: boolean;
}) {
	return (
		<div className="absolute left-4 top-4 z-20 bg-card border dark:border-white/10 rounded-md shadow-sm overflow-hidden flex flex-row">
			<button
				type="button"
				onClick={() => onSelect("agent")}
				className={cn(
					"flex items-center gap-2 px-2.5 py-1.5 text-xs border-r border-border text-foreground transition-colors",
					isOpen && activeTab === "agent"
						? "bg-accent"
						: "hover:bg-accent",
				)}
			>
				<Icon
					icon={faComment}
					className="text-muted-foreground w-3.5"
				/>
				<span>Agent</span>
			</button>
			<button
				type="button"
				onClick={() => onSelect("versions")}
				className={cn(
					"flex items-center gap-2 px-2.5 py-1.5 text-xs text-foreground transition-colors",
					isOpen && activeTab === "versions"
						? "bg-accent"
						: "hover:bg-accent",
				)}
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

function RightFloatingButtons({
	visible,
	onCreate,
}: {
	visible: boolean;
	onCreate: () => void;
}) {
	return (
		<div
			className={cn(
				"absolute right-4 top-4 z-10 bg-card border dark:border-white/10 rounded-md shadow-sm overflow-hidden flex flex-row transition-all duration-200 ease-out",
				visible
					? "translate-x-0 opacity-100"
					: "translate-x-4 opacity-0 pointer-events-none",
			)}
		>
			<button
				type="button"
				onClick={onCreate}
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
	onCreate,
	visible,
}: {
	actorId: string;
	onClose: () => void;
	onSelectActor: (actorId: string) => void;
	onCreate: () => void;
	visible: boolean;
}) {
	const [searchQuery, setSearchQuery] = useState("");

	useEffect(() => {
		setSearchQuery("");
	}, [actorId]);

	return (
		<div
			className={cn(
				"absolute right-4 top-4 bottom-4 w-[min(720px,calc(100%-2rem))] bg-card border dark:border-white/10 rounded-lg shadow-lg z-10 flex flex-col overflow-hidden transition-transform duration-200 ease-out",
				visible
					? "translate-x-0"
					: "translate-x-[calc(100%+2rem)]",
			)}
		>
			<ResizablePanelGroup direction="horizontal" className="flex-1">
				<ResizablePanel
					defaultSize={38}
					minSize={28}
					maxSize={55}
					className="flex flex-col"
				>
					<ActorListToolbar
						onCreate={onCreate}
						actorId={actorId}
						searchQuery={searchQuery}
						onSearchChange={setSearchQuery}
					/>
					<ActorInstanceList
						actorId={actorId}
						searchQuery={searchQuery}
					/>
				</ResizablePanel>
				<ResizableHandle />
				<ResizablePanel defaultSize={62} minSize={45} className="flex flex-col min-w-0">
					<ActorDetailPlaceholder actorId={actorId} onClose={onClose} />
				</ResizablePanel>
			</ResizablePanelGroup>
		</div>
	);
}

function ActorListToolbar({
	onCreate,
	actorId,
	searchQuery,
	onSearchChange,
}: {
	onCreate: () => void;
	actorId: string;
	searchQuery: string;
	onSearchChange: (value: string) => void;
}) {
	const placeholderIndex = Number.parseInt(
		actorId.replace("placeholder-", ""),
		10,
	);
	const actor = PLACEHOLDERS[placeholderIndex];
	const actorName = actor?.name ?? "Actor";
	const instanceCount = actor?.instances ?? 0;
	const searchPlaceholder = `Search ${instanceCount} instance${instanceCount !== 1 ? "s" : ""}...`;
	return (
		<>
			<div className="flex items-center border-b h-[37px] pl-3 pr-2 gap-2">
				<span className="text-xs font-medium text-foreground truncate">
					{actorName}
				</span>
				<Button
					variant="ghost"
					size="icon-sm"
					onClick={onCreate}
					aria-label="Create instance"
					className="ml-auto h-7 w-7"
				>
					<Icon icon={faPlus} />
				</Button>
			</div>
			<div className="flex items-center gap-1 border-b h-[34px] px-2 bg-muted/30">
				<div className="relative flex-1 min-w-0">
					<Icon
						icon={faMagnifyingGlass}
						className="absolute left-2 top-1/2 -translate-y-1/2 w-3 text-muted-foreground pointer-events-none"
					/>
					<Input
						type="search"
						value={searchQuery}
						onChange={(e) => onSearchChange(e.target.value)}
						placeholder={searchPlaceholder}
						className="h-7 pl-7 pr-2 text-xs border-0 bg-transparent focus-visible:border-0"
						aria-label={searchPlaceholder}
					/>
				</div>
				<ActorListFilter />
			</div>
		</>
	);
}

const ACTOR_FILTERS: { key: string; label: string; defaultOn?: boolean }[] = [
	{ key: "destroyed", label: "Show destroyed" },
	{ key: "ids", label: "Show IDs" },
	{ key: "datacenter", label: "Show Actors Datacenter" },
	{ key: "autowake", label: "Auto-wake Actors on select", defaultOn: true },
];

function ActorListFilter() {
	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					size="icon-sm"
					className="h-7 w-7"
					aria-label="Filter actors"
				>
					<Icon icon={faSliders} />
				</Button>
			</PopoverTrigger>
			<PopoverContent align="end" className="w-72 p-2">
				<div className="flex flex-col">
					{ACTOR_FILTERS.map((f) => (
						<label
							key={f.key}
							className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5 hover:bg-accent/40 cursor-pointer transition-colors"
						>
							<span className="text-sm text-foreground">
								{f.label}
							</span>
							<Switch
								defaultChecked={f.defaultOn}
								className="h-5 w-9 [&>span]:h-4 [&>span]:w-4 [&[data-state=checked]>span]:translate-x-4"
							/>
						</label>
					))}
				</div>
			</PopoverContent>
		</Popover>
	);
}

function ActorInstanceList({
	actorId,
	searchQuery,
}: {
	actorId: string;
	searchQuery: string;
}) {
	const placeholderIndex = Number.parseInt(
		actorId.replace("placeholder-", ""),
		10,
	);
	const actor = PLACEHOLDERS[placeholderIndex];
	const instances = useMemo(
		() =>
			actor
				? generateInstances(
						placeholderIndex,
						actor.instances,
						actor.status,
					)
				: [],
		[placeholderIndex, actor],
	);
	const filteredInstances = useMemo(() => {
		const q = searchQuery.trim().toLowerCase();
		if (!q) return instances;
		return instances.filter(
			(inst) =>
				inst.key.toLowerCase().includes(q) ||
				inst.region.toLowerCase().includes(q),
		);
	}, [instances, searchQuery]);
	const [selectedKey, setSelectedKey] = useState<string | null>(null);

	useEffect(() => {
		setSelectedKey(instances[0]?.key ?? null);
	}, [instances]);

	if (!actor) return null;

	return (
		<ScrollArea className="flex-1">
			{filteredInstances.map((inst) => {
				const isSelected = inst.key === selectedKey;
				return (
					<button
						type="button"
						key={inst.key}
						onClick={() => setSelectedKey(inst.key)}
						className={cn(
							"w-full flex items-center gap-2 pl-2 pr-2.5 py-1 border-b border-l-2 border-l-transparent text-sm cursor-pointer hover:bg-accent/50 text-left",
							isSelected && "bg-accent border-l-primary",
						)}
					>
						<ActorStatusIndicator status={inst.status} />
						<span className="text-xs font-mono truncate flex-1 min-w-0">
							{inst.key}
						</span>
						<span className="text-[11px] text-muted-foreground shrink-0">
							{inst.region}
						</span>
						<span className="text-[11px] text-muted-foreground tabular-nums shrink-0 w-8 text-right">
							{inst.uptime}
						</span>
					</button>
				);
			})}
			{filteredInstances.length === 0 && searchQuery.trim() ? (
				<div className="px-3 py-4 text-xs text-muted-foreground text-center">
					No instances match "{searchQuery}"
				</div>
			) : null}
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

interface InspectorTab {
	value: string;
	label: string;
	icon: typeof faBolt;
	group: "source" | "data" | "runtime";
}

const INSPECTOR_TABS: InspectorTab[] = [
	{ value: "code", label: "Code", icon: faCode, group: "source" },
	{ value: "workflow", label: "Workflow", icon: faDiagramProject, group: "source" },
	{ value: "database", label: "Database", icon: faDatabase, group: "data" },
	{ value: "state", label: "State", icon: faBracketsCurly, group: "data" },
	{ value: "queue", label: "Queue", icon: faInbox, group: "data" },
	{ value: "connections", label: "Connections", icon: faPlug, group: "runtime" },
	{ value: "logs", label: "Logs", icon: faFileLines, group: "runtime" },
	{ value: "metadata", label: "Metadata", icon: faTag, group: "runtime" },
];

const APPLICABLE_TABS: Record<string, Set<string>> = {
	realtime: new Set(["code", "state", "connections", "logs", "metadata"]),
	workflow: new Set(["code", "workflow", "state", "queue", "logs", "metadata"]),
	sqlite: new Set(["code", "database", "state", "logs", "metadata"]),
	vm: new Set(["code", "state", "connections", "logs", "metadata"]),
};

const CODE_THEME_SETTINGS = {
	background: "transparent",
	lineHighlight: "transparent",
	gutterBackground: "transparent",
	gutterBorder: "hsl(var(--border))",
	fontSize: "12px",
} as const;

function InspectorRailButton({
	tab,
	active,
	inactive,
	hasBadge,
	onClick,
}: {
	tab: InspectorTab;
	active: boolean;
	inactive?: boolean;
	hasBadge?: boolean;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			aria-label={tab.label}
			className={cn(
				"relative flex items-center gap-1.5 h-7 px-2 rounded-md text-xs transition-colors shrink-0",
				active
					? "bg-accent text-foreground"
					: inactive
						? "text-muted-foreground/40 hover:bg-accent/40 hover:text-muted-foreground"
						: "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
			)}
		>
			<Icon icon={tab.icon} className="text-xs" />
			<span>{tab.label}</span>
			{hasBadge ? (
				<span className="ml-0.5 size-1.5 rounded-full bg-destructive" />
			) : null}
		</button>
	);
}

function InspectorEmptyState({ tab }: { tab: InspectorTab }) {
	return (
		<div className="flex h-full items-center justify-center p-6">
			<div className="flex flex-col items-center gap-3 max-w-xs text-center">
				<div className="flex items-center justify-center w-12 h-12 rounded-lg border bg-muted/40 text-muted-foreground">
					<Icon icon={tab.icon} className="text-base" />
				</div>
				<div>
					<h3 className="text-sm font-medium text-foreground">
						{tab.label}
					</h3>
					<p className="text-xs text-muted-foreground mt-0.5">
						{tab.label} data will render here when the actor produces it.
					</p>
				</div>
			</div>
		</div>
	);
}

function ActorDetailPlaceholder({
	actorId,
	onClose,
}: { actorId: string; onClose: () => void }) {
	const [inspectorTab, setInspectorTab] = useState("code");
	const [theme] = useMockupTheme();
	const codeTheme = useMemo(
		() =>
			theme === "dark"
				? githubDarkInit({ settings: CODE_THEME_SETTINGS })
				: githubLightInit({ settings: CODE_THEME_SETTINGS }),
		[theme],
	);
	const activeTab = INSPECTOR_TABS.find((t) => t.value === inspectorTab)!;

	const placeholderIndex = Number.parseInt(
		actorId.replace("placeholder-", ""),
		10,
	);
	const placeholder = PLACEHOLDERS[placeholderIndex];
	const actorStatus = placeholder?.status;
	const actorType = placeholder?.actorType;
	const isCrashed =
		actorStatus === "crashed" || actorStatus === "crash-loop";
	const applicable = actorType ? APPLICABLE_TABS[actorType] : null;

	const badgeForTab = (value: string) =>
		isCrashed && value === "metadata";

	return (
		<div className="flex flex-col flex-1 min-h-0">
			<div className="flex items-center justify-end border-b h-[37px] pr-2 shrink-0">
				<Button
					variant="ghost"
					size="icon-sm"
					onClick={onClose}
					aria-label="Close inspector"
					className="h-7 w-7"
				>
					<Icon icon={faXmark} />
				</Button>
			</div>
			<div className="flex items-center gap-0.5 border-b h-[34px] px-1.5 bg-muted/30 shrink-0 overflow-x-auto">
				{INSPECTOR_TABS.map((tab) => (
					<InspectorRailButton
						key={tab.value}
						tab={tab}
						active={tab.value === inspectorTab}
						inactive={
							applicable ? !applicable.has(tab.value) : false
						}
						hasBadge={badgeForTab(tab.value)}
						onClick={() => setInspectorTab(tab.value)}
					/>
				))}
			</div>
			<div className="flex flex-col flex-1 min-w-0">
				<div className="flex-1 min-h-0 relative">
					<div
						className={cn(
							"absolute inset-0 p-3",
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
					{inspectorTab !== "code" ? (
						<InspectorEmptyState tab={activeTab} />
					) : null}
				</div>
			</div>
		</div>
	);
}

// -- Main Layout --

export function LayoutMockup() {
	const [selectedActorId, setSelectedActorId] = useState<string | null>(null);
	const [leftOpen, setLeftOpen] = useState(false);
	const [leftTab, setLeftTab] = useState<"agent" | "versions">("agent");
	const [createOpen, setCreateOpen] = useState(false);

	const handleCreate = useCallback(() => setCreateOpen(true), []);

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

	const handleLeftSelect = useCallback(
		(tab: "agent" | "versions") => {
			if (leftOpen && leftTab === tab) {
				setLeftOpen(false);
				return;
			}
			setLeftTab(tab);
			setLeftOpen(true);
			setLeftMounted(true);
		},
		[leftOpen, leftTab],
	);

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
						selectedActorId={selectedActorId}
						onCreate={handleCreate}
					/>
				</div>

				{/* Left floating buttons always visible; active tab gets highlight when panel is open */}
				<LeftFloatingButtons
					onSelect={handleLeftSelect}
					activeTab={leftTab}
					isOpen={leftOpen}
				/>

				{/* Right floating buttons (visible when right popover is closed) */}
				<RightFloatingButtons
					visible={!selectedActorId}
					onCreate={handleCreate}
				/>

				{/* Left popover (always mounted after first open for animation) */}
				{leftMounted ? (
					<LeftPopover
						tab={leftTab}
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
						onCreate={handleCreate}
						visible={!!selectedActorId}
					/>
				) : null}
			</div>

			<CreateActorDialog
				open={createOpen}
				onOpenChange={setCreateOpen}
			/>
		</div>
	);
}
