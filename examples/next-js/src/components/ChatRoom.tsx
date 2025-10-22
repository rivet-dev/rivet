"use client";

import { createRivetKit } from "@rivetkit/next-js/client";
import { useEffect, useState, useRef } from "react";
import type { Message, registry } from "../rivet/registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: process.env.NEXT_PUBLIC_RIVET_ENDPOINT ?? "http://localhost:3000/api/rivet",
	namespace: process.env.NEXT_PUBLIC_RIVET_NAMESPACE,
	token: process.env.NEXT_PUBLIC_RIVET_TOKEN,
});

// Generate avatar color based on username
const getAvatarColor = (username: string) => {
	const colors = [
		"#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
		"#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9"
	];
	const index = username.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
	return colors[index % colors.length];
};

// Generate initials from username
const getInitials = (username: string) => {
	return username.split(' ').map(name => name[0]).join('').toUpperCase().slice(0, 2);
};

export function ChatRoom() {
	const [roomId, setRoomId] = useState("general");
	const [username, setUsername] = useState("User");
	const [input, setInput] = useState("");
	const [messages, setMessages] = useState<Message[]>([]);
	const [isConnected, setIsConnected] = useState(false);
	const [sidebarOpen, setSidebarOpen] = useState(false);
	const messagesEndRef = useRef<HTMLDivElement>(null);

	const chatRoom = useActor({
		name: "chatRoom",
		key: [roomId],
	});

	useEffect(() => {
		if (chatRoom.connection) {
			setIsConnected(true);
			chatRoom.connection.getHistory().then(setMessages);
		} else {
			setIsConnected(false);
		}
	}, [chatRoom.connection]);

	chatRoom.useEvent("newMessage", (message: Message) => {
		setMessages((prev) => [...prev, message]);
	});

	// Auto-scroll to bottom when new messages arrive
	useEffect(() => {
		messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	const sendMessage = async () => {
		if (chatRoom.connection && input.trim()) {
			await chatRoom.connection.sendMessage(username, input);
			setInput("");
		}
	};

	const handleKeyPress = (e: React.KeyboardEvent) => {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			sendMessage();
		}
	};

	const formatTime = (timestamp: number) => {
		const date = new Date(timestamp);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();

		if (isToday) {
			return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		} else {
			return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		}
	};

	return (
		<div className="app">
			<div className={`sidebar ${sidebarOpen ? 'open' : ''}`}>
				<div className="sidebar-header">
					<div className="logo">
						<h1>Rivet Chat</h1>
					</div>
					<button
						className="close-sidebar"
						onClick={() => setSidebarOpen(false)}
					>
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
							<line x1="18" y1="6" x2="6" y2="18"></line>
							<line x1="6" y1="6" x2="18" y2="18"></line>
						</svg>
					</button>
				</div>

				<div className="user-settings">
					<div className="setting-group">
						<label htmlFor="username">Username</label>
						<input
							id="username"
							type="text"
							value={username}
							onChange={(e) => setUsername(e.target.value)}
							placeholder="Enter your username"
							className="setting-input"
						/>
					</div>
					<div className="setting-group">
						<label htmlFor="room">Room</label>
						<input
							id="room"
							type="text"
							value={roomId}
							onChange={(e) => setRoomId(e.target.value)}
							placeholder="Enter room name"
							className="setting-input"
						/>
					</div>
				</div>

				<div className="connection-status">
					<div className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`}>
						<div className="status-dot"></div>
						<span>{isConnected ? 'Connected' : 'Disconnected'}</span>
					</div>
				</div>
			</div>

			<div className="chat-container">
				<div className="chat-header">
					<button
						className="menu-button"
						onClick={() => setSidebarOpen(true)}
					>
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
							<line x1="3" y1="6" x2="21" y2="6"></line>
							<line x1="3" y1="12" x2="21" y2="12"></line>
							<line x1="3" y1="18" x2="21" y2="18"></line>
						</svg>
					</button>
					<div className="room-info">
						<h2>#{roomId}</h2>
						<p>{messages.length} messages</p>
					</div>
				</div>

				<div className="messages-container">
					{messages.length === 0 ? (
						<div className="empty-state">
							<div className="empty-icon">💭</div>
							<h3>Welcome to #{roomId}</h3>
							<p>Start the conversation by sending a message below.</p>
						</div>
					) : (
						<div className="messages">
							{messages.map((msg, i) => {
								const isCurrentUser = msg.sender === username;
								const prevMessage = i > 0 ? messages[i - 1] : null;
								const showAvatar = !prevMessage || prevMessage.sender !== msg.sender;

								return (
									<div key={i} className={`message-wrapper ${isCurrentUser ? 'own' : 'other'}`}>
										{!isCurrentUser && showAvatar && (
											<div className="message-avatar">
												<div
													className="avatar"
													style={{ backgroundColor: getAvatarColor(msg.sender) }}
												>
													{getInitials(msg.sender)}
												</div>
											</div>
										)}
										{!isCurrentUser && !showAvatar && <div className="avatar-spacer"></div>}

										<div className="message-content">
											{!isCurrentUser && showAvatar && (
												<div className="message-sender">{msg.sender}</div>
											)}
											<div className={`message-bubble ${isCurrentUser ? 'own' : 'other'}`}>
												<div className="message-text">{msg.text}</div>
												<div className="message-time">
													{formatTime(msg.timestamp)}
												</div>
											</div>
										</div>
									</div>
								);
							})}
							<div ref={messagesEndRef} />
						</div>
					)}
				</div>

				<div className="input-container">
					<div className="input-wrapper">
						<input
							value={input}
							onChange={(e) => setInput(e.target.value)}
							onKeyPress={handleKeyPress}
							placeholder="Type a message..."
							disabled={!isConnected}
							className="message-input"
						/>
						<button
							onClick={sendMessage}
							disabled={!isConnected || !input.trim()}
							className="send-button"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
								<line x1="22" y1="2" x2="11" y2="13"></line>
								<polygon points="22,2 15,22 11,13 2,9 22,2"></polygon>
							</svg>
						</button>
					</div>
				</div>
			</div>
		</div>
	);
}
