import { createRivetKit } from "@rivetkit/react";
import { useState, useEffect, useCallback } from "react";
import type { registry } from "../src/actors.ts";
import type { ReceivedMessage } from "../src/actors/sender.ts";
import type { QueueMessage } from "../src/actors/multi-queue.ts";
import type { TimeoutResult } from "../src/actors/timeout.ts";
import type { WorkerState } from "../src/actors/worker.ts";
import type { SelfSenderState } from "../src/actors/self-sender.ts";
import type { KeepAwakeState } from "../src/actors/keep-awake.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

type TabName = "send" | "multi-queue" | "timeout" | "worker" | "self-send" | "keep-awake";

// Generate unique key for actor instances
const instanceKey = `demo-${Date.now()}`;

export function App() {
	const [activeTab, setActiveTab] = useState<TabName>("send");

	return (
		<div className="container">
			<div className="header">
				<h1>Queue Sandbox</h1>
				<p>Explore all the ways to use queues in RivetKit</p>
			</div>

			<div className="tabs">
				<button
					className={activeTab === "send" ? "tab active" : "tab"}
					onClick={() => setActiveTab("send")}
				>
					Send
				</button>
				<button
					className={activeTab === "multi-queue" ? "tab active" : "tab"}
					onClick={() => setActiveTab("multi-queue")}
				>
					Multi-Queue
				</button>
				<button
					className={activeTab === "timeout" ? "tab active" : "tab"}
					onClick={() => setActiveTab("timeout")}
				>
					Timeout
				</button>
				<button
					className={activeTab === "worker" ? "tab active" : "tab"}
					onClick={() => setActiveTab("worker")}
				>
					Worker
				</button>
				<button
					className={activeTab === "self-send" ? "tab active" : "tab"}
					onClick={() => setActiveTab("self-send")}
				>
					Self-Send
				</button>
				<button
					className={activeTab === "keep-awake" ? "tab active" : "tab"}
					onClick={() => setActiveTab("keep-awake")}
				>
					Keep Awake
				</button>
			</div>

			<div className="tab-content">
				{activeTab === "send" && <SendTab />}
				{activeTab === "multi-queue" && <MultiQueueTab />}
				{activeTab === "timeout" && <TimeoutTab />}
				{activeTab === "worker" && <WorkerTab />}
				{activeTab === "self-send" && <SelfSendTab />}
				{activeTab === "keep-awake" && <KeepAwakeTab />}
			</div>
		</div>
	);
}

function SendTab() {
	const [messageText, setMessageText] = useState("Hello, Queue!");
	const [messages, setMessages] = useState<ReceivedMessage[]>([]);

	const actor = useActor({ name: "sender", key: [instanceKey] });

	useEffect(() => {
		if (actor.connection) {
			actor.connection.getMessages().then(setMessages);
		}
	}, [actor.connection]);

	const sendMessage = async () => {
		if (actor.handle) {
			await actor.handle.queue.task.send({ text: messageText });
		}
	};

	const receiveMessage = async () => {
		if (actor.connection) {
			const result = await actor.connection.receiveOne();
			if (result) {
				setMessages((prev) => [...prev, result]);
			}
		}
	};

	const clearMessages = async () => {
		if (actor.connection) {
			await actor.connection.clearMessages();
			setMessages([]);
		}
	};

	return (
		<div className="section">
			<h2>Send Messages to Queue</h2>
			<p className="description">
				Client sends messages to actor queue; actor receives and displays them
			</p>

			<div className="form-group">
				<label>Message:</label>
				<input
					type="text"
					value={messageText}
					onChange={(e) => setMessageText(e.target.value)}
					placeholder="Enter message text"
				/>
			</div>

			<div className="button-group">
				<button onClick={sendMessage} className="primary" disabled={!actor.handle}>
					Send to Queue
				</button>
				<button onClick={receiveMessage} disabled={!actor.connection}>
					Receive Next
				</button>
				<button onClick={clearMessages} className="secondary" disabled={!actor.connection}>
					Clear
				</button>
			</div>

			<div className="result-area">
				<h3>Received Messages ({messages.length})</h3>
				{messages.length === 0 ? (
					<p className="empty">No messages received yet</p>
				) : (
					<ul className="message-list">
						{messages.map((msg, i) => (
							<li key={i}>
								<span className="queue-name">{msg.name}</span>
								<span className="message-body">
									{JSON.stringify(msg.body)}
								</span>
								<span className="timestamp">
									{new Date(msg.receivedAt).toLocaleTimeString()}
								</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}

function MultiQueueTab() {
	const [messages, setMessages] = useState<QueueMessage[]>([]);

	const actor = useActor({ name: "multiQueue", key: [instanceKey] });

	useEffect(() => {
		if (actor.connection) {
			actor.connection.getMessages().then(setMessages);
		}
	}, [actor.connection]);

	const sendToQueue = async (queueName: string) => {
		if (actor.handle) {
			await actor.handle.queue[queueName].send({
				priority: queueName,
				timestamp: Date.now(),
			});
		}
	};

	const receiveFromQueues = async (queues: string[]) => {
		if (actor.connection) {
			const received = await actor.connection.receiveFromQueues(queues, 5);
			if (received.length > 0) {
				setMessages((prev) => [
					...prev,
					...received.map((r: { name: string; body: unknown }) => ({
						name: r.name,
						body: r.body,
					})),
				]);
			}
		}
	};

	const clearMessages = async () => {
		if (actor.connection) {
			await actor.connection.clearMessages();
			setMessages([]);
		}
	};

	return (
		<div className="section">
			<h2>Multi-Queue</h2>
			<p className="description">
				Listen to multiple named queues simultaneously
			</p>

			<div className="button-group">
				<button onClick={() => sendToQueue("high")} className="priority-high" disabled={!actor.handle}>
					Send to HIGH
				</button>
				<button onClick={() => sendToQueue("normal")} className="priority-normal" disabled={!actor.handle}>
					Send to NORMAL
				</button>
				<button onClick={() => sendToQueue("low")} className="priority-low" disabled={!actor.handle}>
					Send to LOW
				</button>
			</div>

			<div className="button-group">
				<button onClick={() => receiveFromQueues(["high", "normal", "low"])} disabled={!actor.connection}>
					Receive All
				</button>
				<button onClick={() => receiveFromQueues(["high", "normal"])} disabled={!actor.connection}>
					Receive High+Normal
				</button>
				<button onClick={() => receiveFromQueues(["high"])} disabled={!actor.connection}>
					Receive High Only
				</button>
				<button onClick={clearMessages} className="secondary" disabled={!actor.connection}>
					Clear
				</button>
			</div>

			<div className="result-area">
				<h3>Received Messages ({messages.length})</h3>
				{messages.length === 0 ? (
					<p className="empty">No messages received yet</p>
				) : (
					<ul className="message-list">
						{messages.map((msg, i) => (
							<li key={i}>
								<span className={`queue-name priority-${msg.name}`}>
									{msg.name.toUpperCase()}
								</span>
								<span className="message-body">
									{JSON.stringify(msg.body)}
								</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}

function TimeoutTab() {
	const [timeoutMs, setTimeoutMs] = useState(3000);
	const [isWaiting, setIsWaiting] = useState(false);
	const [lastResult, setLastResult] = useState<TimeoutResult | null>(null);
	const [waitStartedAt, setWaitStartedAt] = useState<number | null>(null);

	const actor = useActor({ name: "timeout", key: [instanceKey] });

	const waitForMessage = async () => {
		if (actor.connection) {
			setIsWaiting(true);
			setWaitStartedAt(Date.now());
			const result = await actor.connection.waitForMessage(timeoutMs);
			setLastResult(result);
			setIsWaiting(false);
			setWaitStartedAt(null);
		}
	};

	const sendMessage = async () => {
		if (actor.handle) {
			await actor.handle.queue.work.send({ text: "Hello!", sentAt: Date.now() });
		}
	};

	return (
		<div className="section">
			<h2>Timeout</h2>
			<p className="description">
				Demonstrate timeout option when no messages arrive
			</p>

			<div className="form-group">
				<label>Timeout: {(timeoutMs / 1000).toFixed(1)} seconds</label>
				<input
					type="range"
					min="1000"
					max="10000"
					step="500"
					value={timeoutMs}
					onChange={(e) => setTimeoutMs(Number(e.target.value))}
				/>
			</div>

			<div className="button-group">
				<button
					onClick={waitForMessage}
					disabled={isWaiting || !actor.connection}
					className="primary"
				>
					{isWaiting ? "Waiting..." : "Wait for Message"}
				</button>
				<button onClick={sendMessage} disabled={!isWaiting || !actor.handle}>
					Send Message
				</button>
			</div>

			{isWaiting && waitStartedAt && (
				<div className="waiting-indicator">
					<CountdownTimer startedAt={waitStartedAt} timeoutMs={timeoutMs} />
				</div>
			)}

			{lastResult && (
				<div className={`result-box ${lastResult.timedOut ? "timeout" : "success"}`}>
					<h3>{lastResult.timedOut ? "Timed Out" : "Message Received"}</h3>
					{lastResult.message !== undefined && (
						<p>Message: {JSON.stringify(lastResult.message)}</p>
					)}
					<p>Waited: {lastResult.waitedMs}ms</p>
				</div>
			)}
		</div>
	);
}

function CountdownTimer({ startedAt, timeoutMs }: { startedAt: number; timeoutMs: number }) {
	const [remaining, setRemaining] = useState(timeoutMs);

	useEffect(() => {
		const interval = setInterval(() => {
			const elapsed = Date.now() - startedAt;
			setRemaining(Math.max(0, timeoutMs - elapsed));
		}, 100);
		return () => clearInterval(interval);
	}, [startedAt, timeoutMs]);

	const progress = 1 - remaining / timeoutMs;

	return (
		<div className="countdown">
			<div className="progress-bar">
				<div className="progress-fill" style={{ width: `${progress * 100}%` }} />
			</div>
			<span>{(remaining / 1000).toFixed(1)}s remaining</span>
		</div>
	);
}

function WorkerTab() {
	const [state, setState] = useState<WorkerState>({
		status: "idle",
		processed: 0,
		lastJob: null,
	});
	const [jobData, setJobData] = useState("task-1");

	const actor = useActor({ name: "worker", key: [instanceKey] });

	useEffect(() => {
		if (actor.connection) {
			const fetchState = async () => {
				const s = await actor.connection!.getState();
				setState(s);
			};
			fetchState();
			const interval = setInterval(fetchState, 1000);
			return () => clearInterval(interval);
		}
	}, [actor.connection]);

	const submitJob = async () => {
		if (actor.handle) {
			await actor.handle.queue.jobs.send({ id: jobData, submittedAt: Date.now() });
		}
	};

	return (
		<div className="section">
			<h2>Worker</h2>
			<p className="description">
				Run handler consuming queue messages in a loop
			</p>

			<div className="status-display">
				<div className={`status-indicator ${state.status}`}>
					{state.status === "running" ? "Running" : "Idle"}
				</div>
				<div className="stat">
					<span className="stat-label">Processed:</span>
					<span className="stat-value">{state.processed}</span>
				</div>
			</div>

			<div className="form-group">
				<label>Job ID:</label>
				<input
					type="text"
					value={jobData}
					onChange={(e) => setJobData(e.target.value)}
					placeholder="Enter job identifier"
				/>
			</div>

			<div className="button-group">
				<button onClick={submitJob} className="primary" disabled={!actor.handle}>
					Submit Job
				</button>
			</div>

			{state.lastJob !== null && (
				<div className="result-box success">
					<h3>Last Processed Job</h3>
					<pre>{JSON.stringify(state.lastJob, null, 2)}</pre>
				</div>
			)}
		</div>
	);
}

function SelfSendTab() {
	const [state, setState] = useState<SelfSenderState>({
		sentCount: 0,
		receivedCount: 0,
		messages: [],
	});
	const [messageBody, setMessageBody] = useState("self-message");

	const actor = useActor({ name: "selfSender", key: [instanceKey] });

	const refreshState = useCallback(async () => {
		if (actor.connection) {
			const s = await actor.connection.getState();
			setState(s);
		}
	}, [actor.connection]);

	useEffect(() => {
		refreshState();
	}, [refreshState]);

	const sendToSelf = async () => {
		if (actor.handle && actor.connection) {
			await actor.handle.queue.self.send({ content: messageBody, sentAt: Date.now() });
			await actor.connection.incrementSentCount();
			await refreshState();
		}
	};

	const receiveFromSelf = async () => {
		if (actor.connection) {
			await actor.connection.receiveFromSelf();
			await refreshState();
		}
	};

	const clearMessages = async () => {
		if (actor.connection) {
			await actor.connection.clearMessages();
			await refreshState();
		}
	};

	return (
		<div className="section">
			<h2>Self-Send</h2>
			<p className="description">
				Actor sends messages to its own queue via inline client
			</p>

			<div className="stats-row">
				<div className="stat">
					<span className="stat-label">Sent:</span>
					<span className="stat-value">{state.sentCount}</span>
				</div>
				<div className="stat">
					<span className="stat-label">Received:</span>
					<span className="stat-value">{state.receivedCount}</span>
				</div>
			</div>

			<div className="form-group">
				<label>Message Content:</label>
				<input
					type="text"
					value={messageBody}
					onChange={(e) => setMessageBody(e.target.value)}
					placeholder="Enter message content"
				/>
			</div>

			<div className="button-group">
				<button onClick={sendToSelf} className="primary" disabled={!actor.handle || !actor.connection}>
					Send to Self
				</button>
				<button onClick={receiveFromSelf} disabled={!actor.connection}>
					Receive from Self
				</button>
				<button onClick={clearMessages} className="secondary" disabled={!actor.connection}>
					Clear
				</button>
			</div>

			<div className="result-area">
				<h3>Received Messages ({state.messages.length})</h3>
				{state.messages.length === 0 ? (
					<p className="empty">No messages received yet</p>
				) : (
					<ul className="message-list">
						{state.messages.map((msg, i) => (
							<li key={i}>
								<span className="message-body">{JSON.stringify(msg)}</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}

function KeepAwakeTab() {
	const [state, setState] = useState<KeepAwakeState>({
		currentTask: null,
		completedTasks: [],
	});
	const [durationMs, setDurationMs] = useState(3000);

	const actor = useActor({ name: "keepAwake", key: [instanceKey] });

	useEffect(() => {
		if (actor.connection) {
			const fetchState = async () => {
				const s = await actor.connection!.getState();
				setState(s);
			};
			fetchState();
			const interval = setInterval(fetchState, 500);
			return () => clearInterval(interval);
		}
	}, [actor.connection]);

	const submitTask = async () => {
		if (actor.handle) {
			await actor.handle.queue.tasks.send({ durationMs });
		}
	};

	const clearTasks = async () => {
		if (actor.connection) {
			await actor.connection.clearTasks();
			const s = await actor.connection.getState();
			setState(s);
		}
	};

	return (
		<div className="section">
			<h2>Keep Awake</h2>
			<p className="description">
				Consume queue message, then do long-running task wrapped in keepAwake()
			</p>

			<div className="form-group">
				<label>Task Duration: {(durationMs / 1000).toFixed(1)} seconds</label>
				<input
					type="range"
					min="1000"
					max="10000"
					step="500"
					value={durationMs}
					onChange={(e) => setDurationMs(Number(e.target.value))}
				/>
			</div>

			<div className="button-group">
				<button onClick={submitTask} className="primary" disabled={!actor.handle}>
					Submit Task
				</button>
				<button onClick={clearTasks} className="secondary" disabled={!actor.connection}>
					Clear History
				</button>
			</div>

			{state.currentTask && (
				<div className="current-task">
					<h3>Current Task</h3>
					<TaskProgress task={state.currentTask} />
				</div>
			)}

			<div className="result-area">
				<h3>Completed Tasks ({state.completedTasks.length})</h3>
				{state.completedTasks.length === 0 ? (
					<p className="empty">No tasks completed yet</p>
				) : (
					<ul className="task-list">
						{state.completedTasks.map((task) => (
							<li key={task.id}>
								<span className="task-id">{task.id.slice(0, 8)}...</span>
								<span className="task-time">
									Completed: {new Date(task.completedAt).toLocaleTimeString()}
								</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}

function TaskProgress({ task }: { task: { id: string; startedAt: number; durationMs: number } }) {
	const [progress, setProgress] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => {
			const elapsed = Date.now() - task.startedAt;
			setProgress(Math.min(1, elapsed / task.durationMs));
		}, 100);
		return () => clearInterval(interval);
	}, [task.startedAt, task.durationMs]);

	return (
		<div className="task-progress">
			<div className="progress-bar">
				<div className="progress-fill" style={{ width: `${progress * 100}%` }} />
			</div>
			<div className="task-info">
				<span>Task: {task.id.slice(0, 8)}...</span>
				<span>{Math.round(progress * 100)}%</span>
			</div>
		</div>
	);
}
