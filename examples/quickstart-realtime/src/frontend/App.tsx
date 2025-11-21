import { createRivetKit } from "@rivetkit/react";
import { useState } from "react";
import type { CursorPosition } from "../backend/registry";

const { useActor } = createRivetKit("http://localhost:6420");

// Generate a simple color from userId for visualization
function getUserColor(userId: string): string {
	let hash = 0;
	for (let i = 0; i < userId.length; i++) {
		hash = userId.charCodeAt(i) + ((hash << 5) - hash);
	}
	const hue = hash % 360;
	return `hsl(${hue}, 70%, 50%)`;
}

export function App() {
	const [userId] = useState(() => `user-${Math.random().toString(36).substr(2, 9)}`);
	const [cursors, setCursors] = useState<CursorPosition[]>([]);

	// Connect to the cursor room
	const cursorRoom = useActor({
		name: "cursorRoom",
		key: ["main"],
	});

	// Listen for cursor moved events from other users
	cursorRoom.useEvent("cursorMoved", (cursor: CursorPosition) => {
		setCursors((prev) => {
			// Update or add the cursor
			const filtered = prev.filter((c) => c.userId !== cursor.userId);
			return [...filtered, cursor];
		});
	});

	// Handle mouse movement
	const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
		if (cursorRoom.connection) {
			const rect = e.currentTarget.getBoundingClientRect();
			const x = e.clientX - rect.left;
			const y = e.clientY - rect.top;
			cursorRoom.connection.updateCursor(userId, x, y);
		}
	};

	return (
		<div className="app-container">
			<div className="header">
				<h1>Quickstart: Realtime</h1>
				<div className="status">
					{cursorRoom.connection ? (
						<span className="connected">Connected</span>
					) : (
						<span className="connecting">Connecting...</span>
					)}
				</div>
				<div className="info">
					Your ID: <span className="user-id" style={{ color: getUserColor(userId) }}>{userId}</span>
				</div>
			</div>
			<div className="canvas" onMouseMove={handleMouseMove}>
				{cursors.map((cursor) => (
					<div
						key={cursor.userId}
						className="cursor"
						style={{
							left: `${cursor.x}px`,
							top: `${cursor.y}px`,
							backgroundColor: getUserColor(cursor.userId),
						}}
					>
						<div className="cursor-label">{cursor.userId}</div>
					</div>
				))}
			</div>
		</div>
	);
}
