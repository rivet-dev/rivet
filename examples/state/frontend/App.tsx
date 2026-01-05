import { createRivetKit } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type { Message, registry } from "../src/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

export function App() {
	const [username, setUsername] = useState("User");
	const [messageText, setMessageText] = useState("");
	const [messages, setMessages] = useState<Message[]>([]);
	const messagesEndRef = useRef<HTMLDivElement>(null);

	const chatRoom = useActor({
		name: "chatRoom",
		key: ["lobby"],
	});

	// Load initial messages when connected
	useEffect(() => {
		if (chatRoom.connection) {
			chatRoom.connection.getMessages().then(setMessages);
		}
	}, [chatRoom.connection]);

	// Listen for new messages
	chatRoom.useEvent("newMessage", (message: Message) => {
		setMessages((prev) => [...prev, message]);
	});

	// Listen for messages cleared event
	chatRoom.useEvent("messagesCleared", () => {
		setMessages([]);
	});

	// Auto-scroll to bottom when new messages arrive
	useEffect(() => {
		messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	const sendMessage = async () => {
		if (chatRoom.connection && messageText.trim()) {
			await chatRoom.connection.sendMessage(username, messageText);
			setMessageText("");
		}
	};

	const clearMessages = async () => {
		if (chatRoom.connection) {
			await chatRoom.connection.clearMessages();
		}
	};

	const handleKeyPress = (e: React.KeyboardEvent) => {
		if (e.key === "Enter") {
			sendMessage();
		}
	};

	return (
		<div className="chat-container">
			<div className="header">
				<h1>Quickstart: State</h1>
				<p>A simple chat room demonstrating persistent state</p>
			</div>

			<div className="user-input">
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
					messages.map((msg) => (
						<div key={msg.id} className="message">
							<div className="message-header">
								<span className="message-sender">{msg.sender}</span>
								<span className="message-timestamp">
									{new Date(msg.timestamp).toLocaleTimeString()}
								</span>
							</div>
							<div className="message-text">{msg.text}</div>
						</div>
					))
				)}
				<div ref={messagesEndRef} />
			</div>

			<div className="input-area">
				<input
					type="text"
					value={messageText}
					onChange={(e) => setMessageText(e.target.value)}
					onKeyPress={handleKeyPress}
					placeholder="Type a message..."
					disabled={!chatRoom.connection}
				/>
				<button
					onClick={sendMessage}
					disabled={!chatRoom.connection || !messageText.trim()}
				>
					Send
				</button>
				<button
					onClick={clearMessages}
					disabled={!chatRoom.connection}
					className="clear-button"
				>
					Clear
				</button>
			</div>
		</div>
	);
}
