import { createRivetKit } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type {
	CursorPosition,
	TextLabel,
	registry,
} from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

// Generate a random user ID
const generateUserId = () =>
	`user-${Math.random().toString(36).substring(2, 9)}`;

// Cursor colors for different users (darker palette)
const CURSOR_COLORS = [
	"#E63946",
	"#2A9D8F",
	"#1B8AAE",
	"#F77F00",
	"#06A77D",
	"#D4A017",
	"#9B59B6",
	"#5DADE2",
];

function getColorForUser(userId: string): string {
	let hash = 0;
	for (let i = 0; i < userId.length; i++) {
		hash = userId.charCodeAt(i) + ((hash << 5) - hash);
	}
	return CURSOR_COLORS[Math.abs(hash) % CURSOR_COLORS.length];
}

// Virtual canvas size - all coordinates are in this space
const CANVAS_WIDTH = 1920;
const CANVAS_HEIGHT = 1080;

export function App() {
	const [roomId, setRoomId] = useState("general");
	const [userId] = useState(generateUserId());
	const [cursors, setCursors] = useState<Record<string, CursorPosition>>({});
	const [textLabels, setTextLabels] = useState<TextLabel[]>([]);
	const [textInput, setTextInput] = useState("");
	const [isTyping, setIsTyping] = useState(false);
	const [typingPosition, setTypingPosition] = useState({ x: 0, y: 0 });
	const [currentTextId, setCurrentTextId] = useState<string | null>(null);
	const [scale, setScale] = useState(1);
	const canvasRef = useRef<HTMLDivElement>(null);
	const containerRef = useRef<HTMLDivElement>(null);

	const cursorRoom = useActor({
		name: "cursorRoom",
		key: [roomId],
	});

	// Calculate scale factor to fit canvas in viewport
	useEffect(() => {
		const updateScale = () => {
			if (!containerRef.current) return;

			const containerWidth = containerRef.current.clientWidth;
			const containerHeight = containerRef.current.clientHeight;

			// Calculate scale to fit canvas while maintaining aspect ratio
			const scaleX = containerWidth / CANVAS_WIDTH;
			const scaleY = containerHeight / CANVAS_HEIGHT;
			const newScale = Math.min(scaleX, scaleY);

			setScale(newScale);
		};

		updateScale();
		window.addEventListener("resize", updateScale);
		return () => window.removeEventListener("resize", updateScale);
	}, []);

	// Load initial state
	useEffect(() => {
		if (cursorRoom.connection) {
			cursorRoom.connection.getRoomState().then((state) => {
				setCursors(state.cursors);
				setTextLabels(state.textLabels);
			}).catch((error) => {
				console.error('error loading room state', error);
			});
		}
	}, [cursorRoom.connection]);

	// Listen for cursor movements
	cursorRoom.useEvent("cursorMoved", (cursor: CursorPosition) => {
		setCursors((prev) => ({
			...prev,
			[cursor.userId]: cursor,
		}));
	});

	// Listen for text updates
	cursorRoom.useEvent("textUpdated", (label: TextLabel) => {
		setTextLabels((prev) => {
			const existingIndex = prev.findIndex(l => l.id === label.id);
			if (existingIndex >= 0) {
				const newLabels = [...prev];
				newLabels[existingIndex] = label;
				return newLabels;
			} else {
				return [...prev, label];
			}
		});
	});

	// Listen for text removal
	cursorRoom.useEvent("textRemoved", (id: string) => {
		setTextLabels((prev) => prev.filter(label => label.id !== id));
	});

	// Listen for cursor removal (when connection closes)
	cursorRoom.useEvent("cursorRemoved", (cursor: CursorPosition) => {
		setCursors((prev) => {
			const newCursors = { ...prev };
			delete newCursors[cursor.userId];
			return newCursors;
		});
	});

	// Convert screen coordinates to canvas coordinates
	const screenToCanvas = (screenX: number, screenY: number) => {
		if (!canvasRef.current) return { x: 0, y: 0 };

		const rect = canvasRef.current.getBoundingClientRect();
		const x = (screenX - rect.left) / scale;
		const y = (screenY - rect.top) / scale;

		return { x, y };
	};

	// Handle mouse movement on canvas
	const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
		if (cursorRoom.connection && canvasRef.current) {
			const { x, y } = screenToCanvas(e.clientX, e.clientY);
			cursorRoom.connection.updateCursor(userId, x, y);
		}
	};

	// Handle canvas click
	const handleCanvasClick = (e: React.MouseEvent<HTMLDivElement>) => {
		if (!canvasRef.current) return;

		const { x, y } = screenToCanvas(e.clientX, e.clientY);
		const newTextId = `${userId}-${Date.now()}`;
		setTypingPosition({ x, y });
		setCurrentTextId(newTextId);
		setIsTyping(true);
		setTextInput("");
	};

	// Handle text input changes
	const handleTextChange = (newText: string) => {
		setTextInput(newText);
		if (cursorRoom.connection && currentTextId && newText.trim()) {
			cursorRoom.connection.updateText(
				currentTextId,
				userId,
				newText,
				typingPosition.x,
				typingPosition.y,
			);
		}
	};

	// Handle key press while typing
	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter") {
			// Finalize the text
			if (textInput.trim() && cursorRoom.connection && currentTextId) {
				cursorRoom.connection.updateText(
					currentTextId,
					userId,
					textInput,
					typingPosition.x,
					typingPosition.y,
				);
			} else if (cursorRoom.connection && currentTextId) {
				// Remove empty text
				cursorRoom.connection.removeText(currentTextId);
			}
			setTextInput("");
			setIsTyping(false);
			setCurrentTextId(null);
		} else if (e.key === "Escape") {
			// Cancel typing and remove text
			if (cursorRoom.connection && currentTextId) {
				cursorRoom.connection.removeText(currentTextId);
			}
			setTextInput("");
			setIsTyping(false);
			setCurrentTextId(null);
		}
	};

	// Cursor is automatically removed when connection closes via connState cleanup

	return (
		<div className="app-container">
			<div className="controls">
				<div className="control-group">
					<label>Room:</label>
					<input
						type="text"
						value={roomId}
						onChange={(e) => setRoomId(e.target.value)}
						placeholder="Enter room name"
					/>
				</div>
				<div className="user-info">
					Your ID: <span style={{ color: getColorForUser(userId) }}>{userId}</span>
				</div>
			</div>

			<div ref={containerRef} className="canvas-container">
				<div
					ref={canvasRef}
					className="canvas"
					style={{
						width: `${CANVAS_WIDTH}px`,
						height: `${CANVAS_HEIGHT}px`,
						transform: `translate(-50%, -50%) scale(${scale})`,
					}}
					onMouseMove={handleMouseMove}
					onClick={handleCanvasClick}
					tabIndex={0}
					onKeyDown={handleKeyDown}
				>
					{/* Render text labels */}
					{textLabels
						.filter((label) => label.id !== currentTextId)
						.map((label) => (
							<div
								key={label.id}
								className="text-label"
								style={{
									left: label.x,
									top: label.y,
									color: getColorForUser(label.userId),
								}}
							>
								{label.text}
							</div>
						))}

					{/* Render text being typed */}
					{isTyping && (
						<div
							className="typing-container"
							style={{
								left: typingPosition.x,
								top: typingPosition.y,
							}}
						>
							<div
								className="typing-text"
								style={{
									color: getColorForUser(userId),
								}}
							>
								{textInput}
								<span className="typing-cursor">|</span>
							</div>
							<div
								className="enter-hint"
								style={{
									borderColor: getColorForUser(userId),
									color: getColorForUser(userId),
								}}
							>
								enter
							</div>
						</div>
					)}

					{/* Render cursors */}
					{Object.entries(cursors).map(([id, cursor]) => {
						const color = getColorForUser(cursor.userId);
						const isOwnCursor = id === userId;
						return (
							<div
								key={id}
								className="cursor"
								style={{
									left: cursor.x,
									top: cursor.y,
								}}
							>
								<svg
									width="20"
									height="24"
									viewBox="0 0 20 24"
									fill="none"
									xmlns="http://www.w3.org/2000/svg"
									className="cursor-svg"
								>
									<path
										d="M10 4 L4 18 L16 18 Z"
										fill={color}
										stroke="white"
										strokeWidth="1.5"
										strokeLinecap="round"
										strokeLinejoin="round"
										transform="rotate(-45 10 12)"
									/>
								</svg>
								<div
									className="cursor-label"
									style={{
										backgroundColor: color,
										borderColor: `${color}40`,
									}}
								>
									{isOwnCursor ? "you" : cursor.userId}
								</div>
							</div>
						);
					})}

					{!cursorRoom.connection && (
						<div className="loading-overlay">Connecting to room...</div>
					)}

					{/* Hidden input to capture typing */}
					{isTyping && (
						<input
							type="text"
							className="hidden-input"
							value={textInput}
							onChange={(e) => handleTextChange(e.target.value)}
							onBlur={() => {
								if (!textInput.trim() && cursorRoom.connection && currentTextId) {
									cursorRoom.connection.removeText(currentTextId);
									setCurrentTextId(null);
								}
								setIsTyping(false);
							}}
							autoFocus
						/>
					)}
				</div>
			</div>
		</div>
	);
}
