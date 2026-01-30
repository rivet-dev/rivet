// Workflow types for visualization

export type NameIndex = number;

export interface LoopIterationMarker {
	loop: NameIndex;
	iteration: number;
}

export type PathSegment = NameIndex | LoopIterationMarker;
export type Location = PathSegment[];

export type SleepState = "pending" | "completed" | "interrupted";

export type BranchStatusType =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

export type WorkflowState =
	| "pending"
	| "running"
	| "sleeping"
	| "failed"
	| "completed"
	| "cancelled"
	| "rolling_back";

export interface StepEntry {
	output?: unknown;
	error?: string;
}

export interface LoopEntry {
	state: unknown;
	iteration: number;
	output?: unknown;
}

export interface SleepEntry {
	deadline: number;
	state: SleepState;
}

export interface MessageEntry {
	name: string;
	data: unknown;
}

export interface RollbackCheckpointEntry {
	name: string;
}

export interface BranchStatus {
	status: BranchStatusType;
	output?: unknown;
	error?: string;
}

export interface JoinEntry {
	branches: Record<string, BranchStatus>;
}

export interface RaceEntry {
	winner: string | null;
	branches: Record<string, BranchStatus>;
}

export interface RemovedEntry {
	originalType: EntryKindType;
	originalName?: string;
}

export type EntryKindType =
	| "step"
	| "loop"
	| "sleep"
	| "message"
	| "rollback_checkpoint"
	| "join"
	| "race"
	| "removed";

export type EntryKind =
	| { type: "step"; data: StepEntry }
	| { type: "loop"; data: LoopEntry }
	| { type: "sleep"; data: SleepEntry }
	| { type: "message"; data: MessageEntry }
	| { type: "rollback_checkpoint"; data: RollbackCheckpointEntry }
	| { type: "join"; data: JoinEntry }
	| { type: "race"; data: RaceEntry }
	| { type: "removed"; data: RemovedEntry };

export type EntryStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "retrying";

// Extended type for visualization (includes meta nodes)
export type ExtendedEntryType = EntryKindType | "input" | "output";

export interface Entry {
	id: string;
	location: Location;
	kind: EntryKind;
	dirty: boolean;
	status?: EntryStatus;
	startedAt?: number;
	completedAt?: number;
	retryCount?: number;
	error?: string;
}

export interface HistoryItem {
	key: string;
	entry: Entry;
}

export interface WorkflowHistory {
	workflowId: string;
	state: WorkflowState;
	nameRegistry: string[];
	history: HistoryItem[];
	input?: unknown;
	output?: unknown;
}

// Parsed node for visualization
export interface ParsedNode {
	id: string;
	key: string;
	name: string;
	type: ExtendedEntryType;
	data: unknown;
	locationIndex: number;
	status: EntryStatus;
	startedAt?: number;
	completedAt?: number;
	duration?: number;
	retryCount?: number;
	error?: string;
}

export interface ParsedBranch {
	name: string;
	status: BranchStatusType;
	isWinner?: boolean;
	nodes: ParsedNode[];
	output?: unknown;
	error?: string;
}

export interface ParsedLoop {
	node: ParsedNode;
	iterations: { iteration: number; nodes: ParsedNode[] }[];
}

export interface ParsedJoin {
	node: ParsedNode;
	branches: ParsedBranch[];
}

export interface ParsedRace {
	node: ParsedNode;
	winner: string | null;
	branches: ParsedBranch[];
}

export type ParsedElement =
	| { type: "node"; node: ParsedNode }
	| { type: "loop"; loop: ParsedLoop }
	| { type: "join"; join: ParsedJoin }
	| { type: "race"; race: ParsedRace };

export interface ParsedWorkflow {
	workflowId: string;
	state: WorkflowState;
	elements: ParsedElement[];
}
