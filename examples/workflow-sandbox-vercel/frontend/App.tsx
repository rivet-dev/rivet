import { useState, useEffect } from "react";
import { createRivetKit } from "@rivetkit/react";
import type {
	registry,
	Order,
	Timer,
	BatchJob,
	ApprovalRequest,
	DashboardState,
	RaceTask,
	Transaction,
} from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`
);

// localStorage helpers for persisting actor keys across page refreshes
function usePersistedState<T>(key: string, initial: T): [T, React.Dispatch<React.SetStateAction<T>>] {
	const [state, setState] = useState<T>(() => {
		const stored = localStorage.getItem(key);
		return stored ? JSON.parse(stored) : initial;
	});

	useEffect(() => {
		localStorage.setItem(key, JSON.stringify(state));
	}, [key, state]);

	return [state, setState];
}

type Tab =
	| "steps"
	| "sleep"
	| "loops"
	| "listen"
	| "join"
	| "race"
	| "rollback";

const TABS: { id: Tab; label: string; description: string }[] = [
	{ id: "steps", label: "Steps", description: "Multi-step order processing" },
	{ id: "sleep", label: "Sleep", description: "Durable countdown timers" },
	{ id: "loops", label: "Loops", description: "Batch processing with cursor" },
	{ id: "listen", label: "Listen", description: "Approval queue with timeout" },
	{ id: "join", label: "Join", description: "Parallel data aggregation" },
	{ id: "race", label: "Race", description: "Work vs timeout pattern" },
	{
		id: "rollback",
		label: "Rollback",
		description: "Compensating transactions",
	},
];

export function App() {
	const [activeTab, setActiveTab] = useState<Tab>("steps");

	return (
		<div className="app">
			<header>
				<h1>Workflow Sandbox</h1>
				<p>Test different RivetKit workflow patterns</p>
			</header>

			<nav className="tabs">
				{TABS.map((tab) => (
					<button
						key={tab.id}
						className={`tab ${activeTab === tab.id ? "active" : ""}`}
						onClick={() => setActiveTab(tab.id)}
					>
						<span className="tab-label">{tab.label}</span>
						<span className="tab-desc">{tab.description}</span>
					</button>
				))}
			</nav>

			<main>
				{activeTab === "steps" && <StepsDemo />}
				{activeTab === "sleep" && <SleepDemo />}
				{activeTab === "loops" && <LoopsDemo />}
				{activeTab === "listen" && <ListenDemo />}
				{activeTab === "join" && <JoinDemo />}
				{activeTab === "race" && <RaceDemo />}
				{activeTab === "rollback" && <RollbackDemo />}
			</main>
		</div>
	);
}

// ============================================================================
// STEPS DEMO - One actor per order
// ============================================================================

function StepsDemo() {
	const [orderKeys, setOrderKeys] = usePersistedState<string[]>("workflow-sandbox:orders", []);

	const createOrder = () => {
		const orderId = `ORD-${Date.now().toString(36).toUpperCase()}`;
		setOrderKeys((prev) => [orderId, ...prev]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Steps Demo</h2>
				<p>
					Sequential workflow steps with automatic retries. Each order goes
					through validate, charge, and fulfill steps.
				</p>
				<button className="primary" onClick={createOrder}>
					Create Order
				</button>
			</div>

			<div className="demo-content">
				<div className="orders-list">
					{orderKeys.length === 0 && (
						<p className="empty">No orders yet. Create one to see the workflow!</p>
					)}
					{orderKeys.map((orderId) => (
						<OrderCard key={orderId} orderId={orderId} />
					))}
				</div>
			</div>
		</div>
	);
}

function OrderCard({ orderId }: { orderId: string }) {
	const actor = useActor({ name: "order", key: [orderId] });
	const [order, setOrder] = useState<Order | null>(null);

	useEffect(() => {
		actor.connection?.getOrder().then(setOrder);
	}, [actor.connection]);

	actor.useEvent("orderUpdated", setOrder);

	const getStatusColor = (status: Order["status"]) => {
		switch (status) {
			case "pending":
				return "#8e8e93";
			case "validating":
			case "charging":
			case "fulfilling":
				return "#ff9f0a";
			case "completed":
				return "#30d158";
			case "failed":
				return "#ff3b30";
			default:
				return "#8e8e93";
		}
	};

	if (!order) return <div className="order-card loading">Loading...</div>;

	return (
		<div className="order-card">
			<div className="order-header">
				<span className="order-id">{order.id}</span>
				<span
					className="status-badge"
					style={{ backgroundColor: getStatusColor(order.status) }}
				>
					{order.status}
				</span>
			</div>
			<div className="order-progress">
				{["Validate", "Charge", "Fulfill", "Complete"].map((step, idx) => (
					<div
						key={step}
						className={`step ${order.step > idx ? "done" : ""} ${order.step === idx + 1 ? "active" : ""}`}
					>
						<div className="step-indicator">
							{order.step > idx ? "✓" : idx + 1}
						</div>
						<span>{step}</span>
					</div>
				))}
			</div>
			{order.error && <div className="error">{order.error}</div>}
		</div>
	);
}

// ============================================================================
// SLEEP DEMO - One actor per timer
// ============================================================================

function SleepDemo() {
	const [timerKeys, setTimerKeys] = usePersistedState<
		{ id: string; name: string; durationMs: number }[]
	>("workflow-sandbox:timers", []);
	const [duration, setDuration] = useState(10);

	const createTimer = () => {
		const timerId = crypto.randomUUID();
		const name = `Timer ${timerKeys.length + 1}`;
		setTimerKeys((prev) => [{ id: timerId, name, durationMs: duration * 1000 }, ...prev]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Sleep Demo</h2>
				<p>
					Durable sleep that survives restarts. Set a timer and watch it
					countdown - even if you refresh the page!
				</p>
				<div className="controls">
					<input
						type="number"
						min="1"
						max="60"
						value={duration}
						onChange={(e) => setDuration(parseInt(e.target.value) || 1)}
					/>
					<span>seconds</span>
					<button className="primary" onClick={createTimer}>
						Create Timer
					</button>
				</div>
			</div>

			<div className="demo-content">
				<div className="timers-list">
					{timerKeys.length === 0 && (
						<p className="empty">No timers yet. Create one to test durable sleep!</p>
					)}
					{timerKeys.map((t) => (
						<TimerCard key={t.id} timerId={t.id} name={t.name} durationMs={t.durationMs} />
					))}
				</div>
			</div>
		</div>
	);
}

function TimerCard({
	timerId,
	name,
	durationMs,
}: {
	timerId: string;
	name: string;
	durationMs: number;
}) {
	const actor = useActor({
		name: "timer",
		key: [timerId],
		createWithInput: { name, durationMs },
	});
	const [timer, setTimer] = useState<Timer | null>(null);
	const [remaining, setRemaining] = useState<number | null>(null);

	useEffect(() => {
		actor.connection?.getTimer().then(setTimer);
	}, [actor.connection]);

	actor.useEvent("timerStarted", setTimer);
	actor.useEvent("timerCompleted", setTimer);

	useEffect(() => {
		if (!timer) return;
		if (timer.completedAt) {
			setRemaining(0);
			return;
		}

		const update = () => {
			const elapsed = Date.now() - timer.startedAt;
			const left = Math.max(0, timer.durationMs - elapsed);
			setRemaining(left);
		};

		update();
		const interval = setInterval(update, 100);
		return () => clearInterval(interval);
	}, [timer]);

	if (!timer || remaining === null) return <div className="timer-card loading">Loading...</div>;

	const isComplete = !!timer.completedAt;
	const progress = isComplete
		? 100
		: ((timer.durationMs - remaining) / timer.durationMs) * 100;

	return (
		<div className={`timer-card ${isComplete ? "complete" : ""}`}>
			<div className="timer-header">
				<span className="timer-name">{timer.name}</span>
				<span className={`timer-status ${isComplete ? "done" : "running"}`}>
					{isComplete ? "Completed" : `${Math.ceil(remaining / 1000)}s`}
				</span>
			</div>
			<div className="progress-bar">
				<div className="progress-fill" style={{ width: `${progress}%` }} />
			</div>
		</div>
	);
}

// ============================================================================
// LOOPS DEMO - One actor per batch job
// ============================================================================

function LoopsDemo() {
	const [jobKeys, setJobKeys] = usePersistedState<string[]>("workflow-sandbox:jobs", []);

	const startJob = () => {
		const jobId = `JOB-${Date.now().toString(36).toUpperCase()}`;
		setJobKeys((prev) => [jobId, ...prev]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Loops Demo</h2>
				<p>
					Batch processing with persistent cursor state. Process 50 items in
					batches of 5 - state persists across restarts.
				</p>
				<button className="primary" onClick={startJob}>
					Start New Batch Job
				</button>
			</div>

			<div className="demo-content">
				<div className="jobs-list">
					{jobKeys.length === 0 && (
						<p className="empty">No batch jobs yet. Start one to see loop processing!</p>
					)}
					{jobKeys.map((jobId) => (
						<BatchJobCard key={jobId} jobId={jobId} />
					))}
				</div>
			</div>
		</div>
	);
}

function BatchJobCard({ jobId }: { jobId: string }) {
	const actor = useActor({
		name: "batch",
		key: [jobId],
		createWithInput: { totalItems: 50, batchSize: 5 },
	});
	const [job, setJob] = useState<BatchJob | null>(null);

	useEffect(() => {
		actor.connection?.getJob().then(setJob);
	}, [actor.connection]);

	actor.useEvent("stateChanged", setJob);

	if (!job) return <div className="job-card loading">Loading...</div>;

	return (
		<div className={`job-card ${job.status}`}>
			<div className="job-header">
				<span className="job-id">{job.id}</span>
				<span className={`job-status ${job.status}`}>{job.status}</span>
			</div>
			<div className="batch-stats">
				<div className="stat">
					<span className="stat-value">{job.processedTotal}</span>
					<span className="stat-label">Items</span>
				</div>
				<div className="stat">
					<span className="stat-value">{job.batches.length}</span>
					<span className="stat-label">Batches</span>
				</div>
			</div>
			<div className="progress-bar large">
				<div
					className="progress-fill"
					style={{ width: `${(job.processedTotal / job.totalItems) * 100}%` }}
				/>
			</div>
		</div>
	);
}

// ============================================================================
// LISTEN DEMO - One actor per approval request
// ============================================================================

function ListenDemo() {
	const [requestKeys, setRequestKeys] = usePersistedState<
		{ id: string; title: string; description: string }[]
	>("workflow-sandbox:requests", []);

	const submitRequest = () => {
		const requestId = crypto.randomUUID();
		const title = `Request ${requestKeys.length + 1}`;
		setRequestKeys((prev) => [
			{ id: requestId, title, description: "Please approve this request" },
			...prev,
		]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Listen Demo</h2>
				<p>
					Approval workflow with 30-second timeout. Submit a request and
					approve/reject it before it times out.
				</p>
				<button className="primary" onClick={submitRequest}>
					Submit Request
				</button>
			</div>

			<div className="demo-content">
				<div className="requests-list">
					{requestKeys.length === 0 && (
						<p className="empty">No requests yet. Submit one to test listen!</p>
					)}
					{requestKeys.map((r) => (
						<ApprovalRequestCard
							key={r.id}
							requestId={r.id}
							title={r.title}
							description={r.description}
						/>
					))}
				</div>
			</div>
		</div>
	);
}

function ApprovalRequestCard({
	requestId,
	title,
	description,
}: {
	requestId: string;
	title: string;
	description: string;
}) {
	const actor = useActor({
		name: "approval",
		key: [requestId],
		createWithInput: { title, description },
	});
	const [request, setRequest] = useState<ApprovalRequest | null>(null);

	useEffect(() => {
		actor.connection?.getRequest().then(setRequest);
	}, [actor.connection]);

	actor.useEvent("requestCreated", setRequest);
	actor.useEvent("requestUpdated", setRequest);

	const getStatusColor = (status: ApprovalRequest["status"]) => {
		switch (status) {
			case "pending":
				return "#ff9f0a";
			case "approved":
				return "#30d158";
			case "rejected":
				return "#ff3b30";
			case "timeout":
				return "#8e8e93";
			default:
				return "#8e8e93";
		}
	};

	if (!request) return <div className="request-card loading">Loading...</div>;

	const isPending = request.status === "pending" && !request.deciding;
	const isDeciding = request.status === "pending" && request.deciding;

	return (
		<div className="request-card">
			<div className="request-header">
				<span className="request-title">{request.title}</span>
				<span
					className="status-badge"
					style={{ backgroundColor: isDeciding ? "#007aff" : getStatusColor(request.status) }}
				>
					{isDeciding ? "processing..." : request.status}
				</span>
			</div>
			{isPending && (
				<RequestCountdown
					request={request}
					onApprove={(approver) => actor.connection?.approve(approver)}
					onReject={(approver) => actor.connection?.reject(approver)}
				/>
			)}
			{request.decidedBy && (
				<div className="decided-by">Decided by: {request.decidedBy}</div>
			)}
		</div>
	);
}

function RequestCountdown({
	request,
	onApprove,
	onReject,
}: {
	request: ApprovalRequest;
	onApprove: (approver: string) => void;
	onReject: (approver: string) => void;
}) {
	const [remaining, setRemaining] = useState(30);

	useEffect(() => {
		const update = () => {
			const elapsed = Date.now() - request.createdAt;
			const left = Math.max(0, 30000 - elapsed);
			setRemaining(Math.ceil(left / 1000));
		};

		update();
		const interval = setInterval(update, 1000);
		return () => clearInterval(interval);
	}, [request.createdAt]);

	return (
		<div className="request-actions">
			<span className="countdown">{remaining}s remaining</span>
			<button className="approve" onClick={() => onApprove("Admin")}>
				Approve
			</button>
			<button className="reject" onClick={() => onReject("Admin")}>
				Reject
			</button>
		</div>
	);
}

// ============================================================================
// JOIN DEMO - Single dashboard actor
// ============================================================================

function JoinDemo() {
	const actor = useActor({ name: "dashboard", key: ["main"] });
	const [state, setState] = useState<DashboardState>({
		data: null,
		loading: false,
		branches: { users: "pending", orders: "pending", metrics: "pending" },
		lastRefresh: null,
	});

	useEffect(() => {
		actor.connection?.getState().then(setState);
	}, [actor.connection]);

	actor.useEvent("stateChanged", setState);

	const getBranchColor = (status: string) => {
		switch (status) {
			case "pending":
				return "#8e8e93";
			case "running":
				return "#ff9f0a";
			case "completed":
				return "#30d158";
			case "failed":
				return "#ff3b30";
			default:
				return "#8e8e93";
		}
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Join Demo</h2>
				<p>
					Parallel data fetching with join (wait-all). Fetch users, orders, and
					metrics simultaneously.
				</p>
				<button
					className="primary"
					onClick={() => actor.connection?.refresh()}
					disabled={state.loading}
				>
					{state.loading ? "Loading..." : "Refresh Dashboard"}
				</button>
			</div>

			<div className="demo-content">
				<div className="branches">
					{(["users", "orders", "metrics"] as const).map((branch) => (
						<div key={branch} className="branch">
							<span
								className="branch-dot"
								style={{ backgroundColor: getBranchColor(state.branches[branch]) }}
							/>
							<span className="branch-name">{branch}</span>
							<span className="branch-status">{state.branches[branch]}</span>
						</div>
					))}
				</div>

				{state.data && (
					<div className="dashboard-data">
						<div className="data-card">
							<h3>Users</h3>
							<div className="data-stat">{state.data.users.count}</div>
							<div className="data-sub">
								{state.data.users.activeToday} active today
							</div>
						</div>
						<div className="data-card">
							<h3>Orders</h3>
							<div className="data-stat">{state.data.orders.count}</div>
							<div className="data-sub">
								${state.data.orders.revenue.toLocaleString()} revenue
							</div>
						</div>
						<div className="data-card">
							<h3>Metrics</h3>
							<div className="data-stat">
								{state.data.metrics.pageViews.toLocaleString()}
							</div>
							<div className="data-sub">page views</div>
						</div>
					</div>
				)}
			</div>
		</div>
	);
}

// ============================================================================
// RACE DEMO - One actor per race task
// ============================================================================

function RaceDemo() {
	const [taskKeys, setTaskKeys] = usePersistedState<
		{ id: string; workDurationMs: number; timeoutMs: number }[]
	>("workflow-sandbox:raceTasks", []);
	const [workDuration, setWorkDuration] = useState(3000);
	const [timeout, setTimeoutVal] = useState(5000);

	const runTask = () => {
		const taskId = crypto.randomUUID();
		setTaskKeys((prev) => [
			{ id: taskId, workDurationMs: workDuration, timeoutMs: timeout },
			...prev,
		]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Race Demo</h2>
				<p>
					Race pattern - work vs timeout. If work completes before timeout, it
					wins. Otherwise timeout wins.
				</p>
				<div className="controls race-controls">
					<div className="control-group">
						<label>Work Duration (ms)</label>
						<input
							type="number"
							value={workDuration}
							onChange={(e) => setWorkDuration(parseInt(e.target.value) || 0)}
						/>
					</div>
					<div className="control-group">
						<label>Timeout (ms)</label>
						<input
							type="number"
							value={timeout}
							onChange={(e) => setTimeoutVal(parseInt(e.target.value) || 0)}
						/>
					</div>
					<button className="primary" onClick={runTask}>
						Run Race
					</button>
				</div>
			</div>

			<div className="demo-content">
				<div className="results-list">
					{taskKeys.map((t) => (
						<RaceTaskCard
							key={t.id}
							taskId={t.id}
							workDurationMs={t.workDurationMs}
							timeoutMs={t.timeoutMs}
						/>
					))}
				</div>
			</div>
		</div>
	);
}

function RaceTaskCard({
	taskId,
	workDurationMs,
	timeoutMs,
}: {
	taskId: string;
	workDurationMs: number;
	timeoutMs: number;
}) {
	const actor = useActor({
		name: "race",
		key: [taskId],
		createWithInput: { workDurationMs, timeoutMs },
	});
	const [task, setTask] = useState<RaceTask | null>(null);
	const [elapsed, setElapsed] = useState(0);
	const [showAnimation, setShowAnimation] = useState(true);

	useEffect(() => {
		actor.connection?.getTask().then(setTask);
	}, [actor.connection]);

	actor.useEvent("raceStarted", setTask);
	actor.useEvent("raceCompleted", setTask);

	// Update elapsed time for animation
	useEffect(() => {
		if (!task) return;

		const updateElapsed = () => {
			const now = task.completedAt ?? Date.now();
			const newElapsed = now - task.startedAt;
			setElapsed(newElapsed);

			// Hide animation once both bars have completed
			const maxDuration = Math.max(task.workDurationMs, task.timeoutMs);
			if (newElapsed > maxDuration + 500) {
				setShowAnimation(false);
			}
		};

		updateElapsed();
		const interval = setInterval(updateElapsed, 50);
		return () => clearInterval(interval);
	}, [task]);

	if (!task) return <div className="result-card loading">Loading...</div>;

	const workProgress = Math.min(100, (elapsed / task.workDurationMs) * 100);
	const timeoutProgress = Math.min(100, (elapsed / task.timeoutMs) * 100);
	const isCompleted = task.status !== "running";

	// Show animation if still running or if recently completed
	if (showAnimation) {
		return (
			<div className="active-race">
				<div className="race-track">
					<div className={`racer work ${isCompleted && task.status === "work_won" ? "winner" : ""}`}>
						<span>Work ({task.workDurationMs}ms)</span>
						<div className="racer-progress">
							<div
								className="racer-bar-fill"
								style={{ width: `${workProgress}%` }}
							/>
						</div>
						{isCompleted && task.status === "work_won" && <span className="winner-badge">Winner!</span>}
					</div>
					<div className={`racer timeout ${isCompleted && task.status === "timeout_won" ? "winner" : ""}`}>
						<span>Timeout ({task.timeoutMs}ms)</span>
						<div className="racer-progress">
							<div
								className="racer-bar-fill timeout-fill"
								style={{ width: `${timeoutProgress}%` }}
							/>
						</div>
						{isCompleted && task.status === "timeout_won" && <span className="winner-badge">Winner!</span>}
					</div>
				</div>
				{isCompleted && (
					<div className="race-result">
						{task.status === "work_won" ? "Work completed first!" : "Timeout triggered!"}
						<span className="actual-duration">{task.actualDurationMs}ms</span>
					</div>
				)}
			</div>
		);
	}

	return (
		<div className={`result-card ${task.status === "work_won" ? "work" : "timeout"}`}>
			<div className="result-winner">
				{task.status === "work_won" ? "Work Won!" : "Timeout!"}
			</div>
			<div className="result-details">
				Work: {task.workDurationMs}ms | Timeout: {task.timeoutMs}ms | Actual:{" "}
				{task.actualDurationMs}ms
			</div>
		</div>
	);
}

// ============================================================================
// ROLLBACK DEMO - One actor per transaction
// ============================================================================

function RollbackDemo() {
	const [txKeys, setTxKeys] = usePersistedState<
		{ id: string; amount: number; shouldFail: boolean }[]
	>("workflow-sandbox:transactions", []);

	const processPayment = (shouldFail: boolean) => {
		const txId = crypto.randomUUID();
		const amount = Math.floor(50 + Math.random() * 200);
		setTxKeys((prev) => [{ id: txId, amount, shouldFail }, ...prev]);
	};

	return (
		<div className="demo">
			<div className="demo-header">
				<h2>Rollback Demo</h2>
				<p>
					Compensating transactions with rollback. Process payments with
					automatic rollback on failure.
				</p>
				<div className="controls">
					<button className="primary" onClick={() => processPayment(false)}>
						Process Payment
					</button>
					<button className="danger" onClick={() => processPayment(true)}>
						Process (Will Fail)
					</button>
				</div>
			</div>

			<div className="demo-content">
				<div className="transactions-list">
					{txKeys.length === 0 && (
						<p className="empty">
							No transactions yet. Process a payment to see rollback!
						</p>
					)}
					{txKeys.map((t) => (
						<TransactionCard
							key={t.id}
							txId={t.id}
							amount={t.amount}
							shouldFail={t.shouldFail}
						/>
					))}
				</div>
			</div>
		</div>
	);
}

function TransactionCard({
	txId,
	amount,
	shouldFail,
}: {
	txId: string;
	amount: number;
	shouldFail: boolean;
}) {
	const actor = useActor({
		name: "payment",
		key: [txId],
		createWithInput: { amount, shouldFail },
	});
	const [tx, setTx] = useState<Transaction | null>(null);

	useEffect(() => {
		actor.connection?.getTransaction().then(setTx);
	}, [actor.connection]);

	actor.useEvent("transactionStarted", setTx);
	actor.useEvent("transactionUpdated", setTx);
	actor.useEvent("transactionCompleted", setTx);
	actor.useEvent("transactionFailed", setTx);

	const getStepColor = (status: string) => {
		switch (status) {
			case "pending":
				return "#8e8e93";
			case "running":
				return "#ff9f0a";
			case "completed":
				return "#30d158";
			case "rolling_back":
				return "#bf5af2";
			case "rolled_back":
				return "#bf5af2";
			case "failed":
				return "#ff3b30";
			default:
				return "#8e8e93";
		}
	};

	const getStepIcon = (status: string) => {
		switch (status) {
			case "pending":
				return "○";
			case "completed":
				return "✓";
			case "rolled_back":
				return "↩";
			default:
				return "○";
		}
	};

	if (!tx) return <div className="transaction-card loading">Loading...</div>;

	const isRollingBack = tx.status === "rolling_back";
	const hasRollback = tx.steps.some((s) => s.status === "rolled_back");

	return (
		<div className={`transaction-card ${tx.status}`}>
			<div className="tx-header">
				<span className="tx-amount">${tx.amount}</span>
				<span className={`tx-status ${tx.status}`}>
					{tx.status === "rolling_back" ? "↩ rolling back" : tx.status}
				</span>
			</div>
			{isRollingBack && (
				<div className="rollback-banner">
					Compensating actions in progress...
				</div>
			)}
			<div className="tx-steps">
				{tx.steps.map((step) => (
					<div key={step.name} className={`tx-step ${step.status}`}>
						<span
							className={`step-icon ${step.status}`}
							style={{ color: getStepColor(step.status) }}
						>
							{getStepIcon(step.status)}
						</span>
						<span className="step-name">{step.name}</span>
						<span className={`step-status ${step.status}`}>
							{step.status}
							{step.status === "rolled_back" && " ↩"}
						</span>
					</div>
				))}
			</div>
			{hasRollback && tx.status === "failed" && (
				<div className="rollback-summary">
					All completed steps have been rolled back
				</div>
			)}
			{tx.error && <div className="error">{tx.error}</div>}
		</div>
	);
}
