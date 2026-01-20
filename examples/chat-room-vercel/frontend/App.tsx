import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { Message, registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${location.origin}/api/rivet`);

export function App() {
	const [roomId, setRoomId] = useState("general");
	const [username, setUsername] = useState("User");
	const [input, setInput] = useState("");
	const [messages, setMessages] = useState<Message[]>([]);

	const chatRoom = useActor({
		name: "chatRoom",
		key: [roomId],
	});

	useEffect(() => {
		if (chatRoom.connection) {
			chatRoom.connection.getHistory().then(setMessages);
		}
	}, [chatRoom.connection]);

	chatRoom.useEvent("newMessage", (message: Message) => {
		setMessages((prev) => [...prev, message]);
	});

	const sendMessage = async () => {
		if (chatRoom.connection && input.trim()) {
			await chatRoom.connection.sendMessage(username, input);
			setInput("");
		}
	};

	const handleKeyPress = (e: React.KeyboardEvent) => {
		if (e.key === "Enter") {
			sendMessage();
		}
	};

	return (
		<div className="chat-container">
			<div className="room-header">
				<h3>Chat Room: {roomId}</h3>
			</div>

			<div className="room-controls">
				<label>Room:</label>
				<input
					type="text"
					value={roomId}
					onChange={(e) => setRoomId(e.target.value)}
					placeholder="Enter room name"
				/>
				<label>Username:</label>
				<input
					type="text"
					value={username}
					onChange={(e) => setUsername(e.target.value)}
					placeholder="Enter your username"
				/>
			</div>

			<div className="messages">
				{messages.length === 0 ? (
					<div className="empty-message">
						No messages yet. Start the conversation!
					</div>
				) : (
					messages.map((msg, i) => (
						<div key={i} className="message">
							<div className="message-sender">{msg.sender}</div>
							<div className="message-text">{msg.text}</div>
							<div className="message-timestamp">
								{new Date(msg.timestamp).toLocaleTimeString()}
							</div>
						</div>
					))
				)}
			</div>

			<div className="input-area">
				<input
					value={input}
					onChange={(e) => setInput(e.target.value)}
					onKeyPress={handleKeyPress}
					placeholder="Type a message..."
					disabled={!chatRoom.connection}
				/>
				<button
					onClick={sendMessage}
					disabled={!chatRoom.connection || !input.trim()}
				>
					Send
				</button>
			</div>
		</div>
	);
}
