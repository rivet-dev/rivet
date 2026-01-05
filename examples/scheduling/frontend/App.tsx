import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { Reminder, Registry } from "../src/registry";

const { useActor } = createRivetKit<Registry>("http://localhost:6420");

export function App() {
	const [reminders, setReminders] = useState<Reminder[]>([]);
	const [message, setMessage] = useState("");
	const [delay, setDelay] = useState(5);
	const [timestamp, setTimestamp] = useState("");
	const [triggeredReminders, setTriggeredReminders] = useState<Reminder[]>([]);
	const [stats, setStats] = useState({ total: 0, completed: 0, pending: 0 });

	const reminderActor = useActor({
		name: "reminderActor",
		key: ["main"],
	});

	// Load initial state
	useEffect(() => {
		if (reminderActor.connection) {
			reminderActor.connection.getReminders().then((initialReminders) => {
				setReminders(initialReminders);
			}).catch((error) => {
				console.error("error loading reminders", error);
			});

			reminderActor.connection.getStats().then((initialStats) => {
				setStats(initialStats);
			}).catch((error) => {
				console.error("error loading stats", error);
			});
		}
	}, [reminderActor.connection]);

	// Listen for reminder triggered events
	reminderActor.useEvent("reminderTriggered", (reminder: Reminder) => {
		// Update the reminders list
		setReminders((prev) =>
			prev.map((r) => (r.id === reminder.id ? reminder : r))
		);

		// Add to triggered notifications
		setTriggeredReminders((prev) => [reminder, ...prev].slice(0, 5));

		// Update stats
		if (reminderActor.connection) {
			reminderActor.connection.getStats().then(setStats).catch((error) => {
				console.error("error loading stats", error);
			});
		}
	});

	// Schedule a reminder with delay
	const handleScheduleReminder = async () => {
		if (!reminderActor.connection || !message.trim()) return;

		const delayMs = delay * 1000;
		const reminder = await reminderActor.connection.scheduleReminder(message, delayMs);
		setReminders((prev) => [...prev, reminder]);
		setMessage("");

		// Update stats
		const newStats = await reminderActor.connection.getStats();
		setStats(newStats);
	};

	// Schedule a reminder at a specific time
	const handleScheduleAt = async () => {
		if (!reminderActor.connection || !message.trim() || !timestamp) return;

		const timestampMs = new Date(timestamp).getTime();
		const reminder = await reminderActor.connection.scheduleReminderAt(message, timestampMs);
		setReminders((prev) => [...prev, reminder]);
		setMessage("");
		setTimestamp("");

		// Update stats
		const newStats = await reminderActor.connection.getStats();
		setStats(newStats);
	};

	// Cancel a reminder
	const handleCancelReminder = async (reminderId: string) => {
		if (!reminderActor.connection) return;

		await reminderActor.connection.cancelReminder(reminderId);
		setReminders((prev) => prev.filter((r) => r.id !== reminderId));

		// Update stats
		const newStats = await reminderActor.connection.getStats();
		setStats(newStats);
	};

	// Calculate time until reminder triggers
	const getTimeUntil = (scheduledAt: number) => {
		const now = Date.now();
		const diff = scheduledAt - now;

		if (diff <= 0) return "now";

		const seconds = Math.floor(diff / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);

		if (hours > 0) return `in ${hours}h ${minutes % 60}m`;
		if (minutes > 0) return `in ${minutes}m ${seconds % 60}s`;
		return `in ${seconds}s`;
	};

	// Clear triggered notifications
	const clearTriggered = () => {
		setTriggeredReminders([]);
	};

	return (
		<div className="app">
			<div className="header">
				<h1>Quickstart: Scheduling</h1>
				<p>Demonstrating c.schedule.after() and c.schedule.at()</p>
			</div>

			<div className="stats-bar">
				<div className="stat">
					<span className="stat-label">Total:</span>
					<span className="stat-value">{stats.total}</span>
				</div>
				<div className="stat">
					<span className="stat-label">Completed:</span>
					<span className="stat-value">{stats.completed}</span>
				</div>
				<div className="stat">
					<span className="stat-label">Pending:</span>
					<span className="stat-value">{stats.pending}</span>
				</div>
			</div>

			{triggeredReminders.length > 0 && (
				<div className="notifications">
					<div className="notifications-header">
						<h3>Recent Notifications</h3>
						<button onClick={clearTriggered} className="clear-btn">Clear</button>
					</div>
					{triggeredReminders.map((reminder) => (
						<div key={reminder.id} className="notification">
							<div className="notification-icon">🔔</div>
							<div className="notification-content">
								<div className="notification-message">{reminder.message}</div>
								<div className="notification-time">
									Triggered at {new Date(reminder.completedAt!).toLocaleTimeString()}
								</div>
							</div>
						</div>
					))}
				</div>
			)}

			<div className="content">
				<div className="section">
					<h2>Schedule Reminder</h2>

					<div className="form-group">
						<label>Message:</label>
						<input
							type="text"
							value={message}
							onChange={(e) => setMessage(e.currentTarget.value)}
							placeholder="Enter reminder message"
							className="input"
						/>
					</div>

					<div className="schedule-options">
						<div className="option">
							<h3>After Delay</h3>
							<div className="form-group">
								<label>Delay (seconds):</label>
								<input
									type="number"
									value={delay}
									onChange={(e) => setDelay(Number(e.currentTarget.value))}
									min="1"
									className="input"
								/>
							</div>
							<button
								onClick={handleScheduleReminder}
								disabled={!message.trim()}
								className="btn btn-primary"
							>
								Schedule Reminder
							</button>
						</div>

						<div className="option">
							<h3>At Specific Time</h3>
							<div className="form-group">
								<label>Date & Time:</label>
								<input
									type="datetime-local"
									value={timestamp}
									onChange={(e) => setTimestamp(e.currentTarget.value)}
									className="input"
								/>
							</div>
							<button
								onClick={handleScheduleAt}
								disabled={!message.trim() || !timestamp}
								className="btn btn-primary"
							>
								Schedule at Time
							</button>
						</div>
					</div>
				</div>

				<div className="section">
					<h2>Reminders</h2>
					{reminders.length === 0 ? (
						<div className="empty-state">No reminders scheduled</div>
					) : (
						<div className="reminders-list">
							{reminders.map((reminder) => (
								<div
									key={reminder.id}
									className={`reminder-item ${reminder.completedAt ? 'completed' : 'pending'}`}
								>
									<div className="reminder-content">
										<div className="reminder-message">{reminder.message}</div>
										<div className="reminder-meta">
											{reminder.completedAt ? (
												<span className="reminder-status completed">
													✓ Completed at {new Date(reminder.completedAt).toLocaleString()}
												</span>
											) : (
												<>
													<span className="reminder-status pending">
														{getTimeUntil(reminder.scheduledAt)}
													</span>
													<span className="reminder-scheduled">
														Scheduled: {new Date(reminder.scheduledAt).toLocaleString()}
													</span>
												</>
											)}
										</div>
									</div>
									{!reminder.completedAt && (
										<button
											onClick={() => handleCancelReminder(reminder.id)}
											className="btn btn-cancel"
										>
											Cancel
										</button>
									)}
								</div>
							))}
						</div>
					)}
				</div>
			</div>

			{!reminderActor.connection && (
				<div className="loading-overlay">Connecting to server...</div>
			)}
		</div>
	);
}
