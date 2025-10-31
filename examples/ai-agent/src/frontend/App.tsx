import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import { registry } from "../backend/registry";
import type { Message } from "../backend/types";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

export function App() {
	const aiAgent = useActor({
		name: "aiAgent",
		key: ["default"],
	});
	const [messages, setMessages] = useState<Message[]>([]);
	const [input, setInput] = useState("");
	const [isLoading, setIsLoading] = useState(false);

	useEffect(() => {
		if (aiAgent.connection) {
			aiAgent.connection.getMessages().then(setMessages);
		}
	}, [aiAgent.connection]);

	aiAgent.useEvent("messageReceived", (message: Message) => {
		setMessages((prev) => [...prev, message]);
		setIsLoading(false);
	});

	const handleSendMessage = async () => {
		if (aiAgent.connection && input.trim()) {
			setIsLoading(true);

			const userMessage = { role: "user", content: input, timestamp: Date.now() } as Message;
			setMessages((prev) => [...prev, userMessage]);

			await aiAgent.connection.sendMessage(input);
			setInput("");
		}
	};

	return (
		<div className="ai-chat">
			<div className="messages">
				{messages.length === 0 ? (
					<div className="empty-message">
						Ask the AI assistant a question to get started
					</div>
				) : (
					messages.map((msg, i) => (
						<div key={i} className={`message ${msg.role}`}>
							<div className="avatar">{msg.role === "user" ? "👤" : "🤖"}</div>
							<div className="content">{msg.content}</div>
						</div>
					))
				)}
				{isLoading && (
					<div className="message assistant loading">
						<div className="avatar">🤖</div>
						<div className="content">Thinking...</div>
					</div>
				)}
			</div>

			<div className="input-area">
				<input
					value={input}
					onChange={(e) => setInput(e.target.value)}
					onKeyPress={(e) => e.key === "Enter" && handleSendMessage()}
					placeholder="Ask the AI assistant..."
					disabled={isLoading}
				/>
				<button
					onClick={handleSendMessage}
					disabled={isLoading || !input.trim()}
				>
					Send
				</button>
			</div>
		</div>
	);
}
