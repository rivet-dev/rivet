"use client";

// TODO: Replace this simple request/response chat with useChat from @ai-sdk/react
// when we have proper streaming support via Rivet actors

import Image from "next/image";

import { PromptInputBasic } from "./chatinput";
import { Markdown } from "./ui/markdown";
import { useState, useEffect, useRef } from "react";
import { ChatContainer } from "./ui/chat-container";
import { UIMessage } from "ai";
import { ToolMessage } from "./tools";
import { CompressedImage } from "@/lib/image-compression";
import { client } from "@/rivet/client";
import { stopStreamAction } from "@/actions/stop-stream";
import { sendChatMessage } from "@/actions/send-chat-message";

export default function Chat(props: {
	appId: string;
	initialMessages: UIMessage[];
	isLoading?: boolean;
	topBar?: React.ReactNode;
	running: boolean;
}) {
	const [messages, setMessages] = useState<UIMessage[]>(props.initialMessages);
	const [isGenerating, setIsGenerating] = useState(props.running);
	const streamConnectionRef = useRef<any>(null);

	// Connect to streamState actor for abort events
	useEffect(() => {
		let mounted = true;

		const setupConnection = async () => {
			try {
				// Get initial status
				const status = await client.streamState
					.getOrCreate([props.appId])
					.getStatus();
				if (mounted) setIsGenerating(status === "running");

				// Connect to actor for events
				const connection = await client.streamState
					.getOrCreate([props.appId])
					.connect();
				if (!mounted) return;

				streamConnectionRef.current = connection;

				// Listen for abort events
				connection.on("abort", () => {
					if (mounted) setIsGenerating(false);
				});
			} catch (err) {
				console.error("Failed to connect to stream state:", err);
			}
		};

		setupConnection();

		return () => {
			mounted = false;
			if (streamConnectionRef.current?.disconnect) {
				streamConnectionRef.current.disconnect();
			}
		};
	}, [props.appId]);

	const [input, setInput] = useState("");

	const handleSendMessage = async (userMessage: UIMessage) => {
		// Add user message to UI
		setMessages((prev) => [...prev, userMessage]);
		setIsGenerating(true);

		try {
			// Send message and get response
			const assistantMessage = await sendChatMessage(props.appId, userMessage);

			// Add assistant message to UI
			setMessages((prev) => [...prev, assistantMessage]);
		} catch (error) {
			console.error("Error sending message:", error);
		} finally {
			setIsGenerating(false);
		}
	};

	const onSubmit = (e?: React.FormEvent<HTMLFormElement>) => {
		if (e?.preventDefault) {
			e.preventDefault();
		}

		const userMessage: UIMessage = {
			id: crypto.randomUUID(),
			role: "user",
			parts: [
				{
					type: "text",
					text: input,
				},
			],
		};

		handleSendMessage(userMessage);
		setInput("");
	};

	const onSubmitWithImages = (text: string, images: CompressedImage[]) => {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const parts: any[] = [];

		if (text.trim()) {
			parts.push({
				type: "text",
				text: text,
			});
		}

		images.forEach((image) => {
			parts.push({
				type: "file",
				mediaType: image.mimeType,
				url: image.data,
			});
		});

		const userMessage: UIMessage = {
			id: crypto.randomUUID(),
			role: "user",
			parts,
		};

		handleSendMessage(userMessage);
		setInput("");
	};

	async function handleStop() {
		await stopStreamAction(props.appId);
	}

	return (
		<div
			className="flex flex-col h-full"
			style={{ transform: "translateZ(0)" }}
		>
			{props.topBar}
			<div
				className="flex-1 overflow-y-auto flex flex-col space-y-6 min-h-0"
				style={{ overflowAnchor: "auto" }}
			>
				<ChatContainer autoScroll>
					{messages.map((message: any) => (
						<MessageBody key={message.id} message={message} />
					))}
				</ChatContainer>
			</div>
			<div className="flex-shrink-0 p-3 transition-all bg-background md:backdrop-blur-sm">
				<PromptInputBasic
					stop={handleStop}
					input={input}
					onValueChange={(value) => {
						setInput(value);
					}}
					onSubmit={onSubmit}
					onSubmitWithImages={onSubmitWithImages}
					isGenerating={props.isLoading || isGenerating}
				/>
			</div>
		</div>
	);
}

function MessageBody({ message }: { message: any }) {
	if (message.role === "user") {
		return (
			<div className="flex justify-end py-1 mb-4">
				<div className="bg-neutral-200 dark:bg-neutral-700 rounded-xl px-4 py-1 max-w-[80%] ml-auto">
					{message.parts.map((part: any, index: number) => {
						if (part.type === "text") {
							return <div key={index}>{part.text}</div>;
						} else if (
							part.type === "file" &&
							part.mediaType?.startsWith("image/")
						) {
							return (
								<div key={index} className="mt-2">
									<Image
										src={part.url as string}
										alt="User uploaded image"
										width={200}
										height={200}
										className="max-w-full h-auto rounded"
										style={{ maxHeight: "200px" }}
									/>
								</div>
							);
						}
						return <div key={index}>unexpected message</div>;
					})}
				</div>
			</div>
		);
	}

	if (Array.isArray(message.parts) && message.parts.length !== 0) {
		return (
			<div className="mb-4">
				{message.parts.map((part: any, index: any) => {
					if (part.type === "text") {
						return (
							<div key={index} className="mb-4">
								<Markdown className="prose prose-sm dark:prose-invert max-w-none">
									{part.text}
								</Markdown>
							</div>
						);
					}

					if (part.type.startsWith("tool-")) {
						return <ToolMessage key={index} toolInvocation={part} />;
					}
				})}
			</div>
		);
	}

	if (message.parts) {
		return (
			<Markdown className="prose prose-sm dark:prose-invert max-w-none">
				{message.parts
					.map((part: any) =>
						part.type === "text" ? part.text : "[something went wrong]"
					)
					.join("")}
			</Markdown>
		);
	}

	return (
		<div>
			<p className="text-gray-500">Something went wrong</p>
		</div>
	);
}
