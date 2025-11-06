import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../../backend/registry";
import { authClient } from "../auth-client";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

interface ChatRoomProps {
	user: { id: string; email: string };
	onSignOut: () => void;
}

export function ChatRoom({ user, onSignOut }: ChatRoomProps) {
	const [message, setMessage] = useState("");
	const [messages, setMessages] = useState<
		Array<{
			id: string;
			userId: string;
			username: string;
			message: string;
			timestamp: number;
		}>
	>([]);
	const [roomId] = useState("general");

	const chatRoom = useActor({
		name: "chatRoom",
		key: [roomId],
	});

	// Listen for new messages
	chatRoom.useEvent("newMessage", (newMessage) => {
		setMessages((prev) => [
			...prev,
			newMessage as {
				id: string;
				userId: string;
				username: string;
				message: string;
				timestamp: number;
			},
		]);
	});

	// Load initial messages when connected
	useEffect(() => {
		if (chatRoom.connection) {
			chatRoom.connection.getMessages().then((initialMessages) => {
				setMessages(initialMessages);
			});
		}
	}, [chatRoom.connection]);

	const handleSendMessage = async (e: React.FormEvent) => {
		e.preventDefault();
		if (!message.trim() || !chatRoom.connection) return;

		try {
			await chatRoom.connection.sendMessage(message.trim());
			setMessage("");
		} catch (error) {
			console.error("Failed to send message:", error);
		}
	};

	const handleSignOut = async () => {
		await authClient.signOut();
		onSignOut();
	};

	return (
		<div style={{ maxWidth: "800px", margin: "0 auto", padding: "20px" }}>
			<div
				style={{
					display: "flex",
					justifyContent: "space-between",
					alignItems: "center",
					marginBottom: "20px",
					paddingBottom: "10px",
					borderBottom: "1px solid #ccc",
				}}
			>
				<div>
					<h2>Chat Room: {roomId}</h2>
					<p>Logged in as: {user.email}</p>
				</div>
				<button
					onClick={handleSignOut}
					style={{
						padding: "8px 16px",
						backgroundColor: "#dc3545",
						color: "white",
						border: "none",
						borderRadius: "4px",
						cursor: "pointer",
					}}
				>
					Sign Out
				</button>
			</div>

			<div
				style={{
					height: "400px",
					overflowY: "auto",
					border: "1px solid #ccc",
					padding: "15px",
					marginBottom: "15px",
					backgroundColor: "#f9f9f9",
				}}
			>
				{messages.length === 0 ? (
					<p style={{ color: "#666", fontStyle: "italic" }}>
						No messages yet. Start the conversation!
					</p>
				) : (
					messages.map((msg) => (
						<div
							key={msg.id}
							style={{
								marginBottom: "10px",
								padding: "8px",
								backgroundColor: msg.userId === user.id ? "#e3f2fd" : "#fff",
								borderRadius: "4px",
								border: "1px solid #ddd",
							}}
						>
							<div
								style={{
									fontSize: "12px",
									color: "#666",
									marginBottom: "2px",
								}}
							>
								{msg.username} • {new Date(msg.timestamp).toLocaleTimeString()}
							</div>
							<div>{msg.message}</div>
						</div>
					))
				)}
			</div>

			<form
				onSubmit={handleSendMessage}
				style={{
					display: "flex",
					gap: "10px",
				}}
			>
				<input
					type="text"
					value={message}
					onChange={(e) => setMessage(e.target.value)}
					placeholder="Type your message..."
					style={{
						flex: 1,
						padding: "10px",
						border: "1px solid #ccc",
						borderRadius: "4px",
					}}
				/>
				<button
					type="submit"
					disabled={!message.trim() || !chatRoom.connection}
					style={{
						padding: "10px 20px",
						backgroundColor: "#007bff",
						color: "white",
						border: "none",
						borderRadius: "4px",
						cursor:
							!message.trim() || !chatRoom.connection
								? "not-allowed"
								: "pointer",
					}}
				>
					Send
				</button>
			</form>

			<div style={{ marginTop: "10px", fontSize: "12px", color: "#666" }}>
				Connection Status: {chatRoom.connection ? "Connected" : "Connecting..."}
			</div>
		</div>
	);
}
