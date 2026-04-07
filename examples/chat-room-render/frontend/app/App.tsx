import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
	Badge,
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Footer,
	FormField,
	GridDecoration,
	Navigation,
	RenderLogo,
	ThemeToggle,
} from "render-dds";
import type { Message } from "../../src/actors.ts";
import { rivetClientBase, type AppRegistry } from "./rivet-client";

export function App() {
	const rivet = useMemo(() => createRivetKit<AppRegistry>(rivetClientBase()), []);
	const { useActor } = rivet;

	const [roomId, setRoomId] = useState("general");
	const [username, setUsername] = useState("User");
	const [input, setInput] = useState("");
	const [messages, setMessages] = useState<Message[]>([]);
	const bottomRef = useRef<HTMLDivElement>(null);

	const chatRoom = useActor({ name: "chatRoom", key: [roomId] });
	const live = Boolean(chatRoom.connection);

	useEffect(() => {
		if (chatRoom.connection) {
			chatRoom.connection.getHistory().then(setMessages);
		}
	}, [chatRoom.connection]);

	chatRoom.useEvent("newMessage", (msg: Message) => {
		setMessages((prev) => [...prev, msg]);
	});

	useEffect(() => {
		bottomRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	const send = async () => {
		if (chatRoom.connection && input.trim()) {
			await chatRoom.connection.sendMessage(username, input);
			setInput("");
		}
	};

	return (
		<div className="relative flex min-h-screen flex-col bg-background text-foreground">
			<GridDecoration position="top-right" className="pointer-events-none" height={280} opacity={0.28} width={280} />

			<Navigation
				className="relative z-10 border-b border-border bg-background/80 backdrop-blur-sm"
				logo={
					<div className="flex items-center gap-3">
						<RenderLogo variant="mark" height={28} />
						<div className="flex flex-col">
							<span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">RivetKit</span>
							<span className="text-sm font-semibold leading-tight text-foreground">Chat Room</span>
						</div>
					</div>
				}
				actions={
					<div className="flex items-center gap-2">
						<Badge variant={live ? "green" : "red-light"}>
							<span className="inline-flex items-center gap-1.5">
								<span className={live ? "size-1.5 rounded-full bg-green-500" : "size-1.5 animate-pulse rounded-full bg-red-400"} />
								{live ? "Connected" : "Connecting"}
							</span>
						</Badge>
						<ThemeToggle size="sm" variant="outline" />
					</div>
				}
			/>

			<main className="relative z-10 mx-auto flex w-full max-w-2xl flex-1 flex-col gap-4 px-4 py-8 sm:px-6">
				<div className="text-center">
					<h1 className="font-sans text-4xl font-bold tracking-tight text-foreground sm:text-5xl">
						Chat Room
					</h1>
					<p className="mx-auto mt-3 max-w-xl text-base text-muted-foreground sm:text-lg">
						Real-time messaging with persistent history — each room is a separate actor instance.
					</p>
				</div>

				<div className="flex gap-3">
					<FormField id="room" label="Room" value={roomId} onChange={(e) => setRoomId(e.target.value)} className="flex-1" />
					<FormField id="username" label="Username" value={username} onChange={(e) => setUsername(e.target.value)} className="flex-1" />
				</div>

				<Card variant="elevated" className="flex flex-1 flex-col overflow-hidden border-border shadow-lg shadow-black/5 dark:shadow-black/20">
					<div className="border-b border-border bg-muted/30 px-5 py-3 dark:bg-muted/15">
						<span className="text-sm font-semibold text-foreground">#{roomId}</span>
					</div>

					<CardContent className="flex-1 overflow-y-auto px-5 py-4" style={{ maxHeight: 420 }}>
						{messages.length === 0 ? (
							<p className="py-10 text-center text-sm italic text-muted-foreground">No messages yet. Start the conversation!</p>
						) : (
							<div className="space-y-3">
								{messages.map((msg, i) => (
									<div key={i} className="rounded-lg border border-border bg-muted/20 px-4 py-3 dark:bg-muted/10">
										<div className="flex items-baseline justify-between">
											<span className="text-sm font-semibold text-primary">{msg.sender}</span>
											<span className="text-xs text-muted-foreground">{new Date(msg.timestamp).toLocaleTimeString()}</span>
										</div>
										<p className="mt-1 text-sm text-foreground">{msg.text}</p>
									</div>
								))}
								<div ref={bottomRef} />
							</div>
						)}
					</CardContent>

					<div className="flex gap-2 border-t border-border px-5 py-4">
						<input
							value={input}
							onChange={(e) => setInput(e.target.value)}
							onKeyDown={(e) => e.key === "Enter" && send()}
							placeholder="Type a message…"
							disabled={!live}
							className="flex-1 rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-primary/40"
						/>
						<Button variant="default" disabled={!live || !input.trim()} onClick={send}>Send</Button>
					</div>
				</Card>
			</main>

			<section className="flex justify-center px-4 pb-10 pt-2 md:pb-14">
				<div className="w-full max-w-md">
					<Card variant="outlined" className="border-dashed border-border/80 text-center">
						<CardHeader className="pb-2">
							<CardTitle className="text-base">Deploy on Render</CardTitle>
						</CardHeader>
						<CardContent className="flex justify-center pt-0">
							<a
								href="https://render.com/deploy?repo=https://github.com/ojusave/chatroom-rivet"
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex shrink-0"
								aria-label="Deploy to Render"
							>
								<img
									src="https://render.com/images/deploy-to-render-button.svg"
									alt=""
									width={155}
									height={40}
									decoding="async"
								/>
							</a>
						</CardContent>
					</Card>
				</div>
			</section>

			<Footer
				centered
				className="relative z-10 mt-auto border-t border-border bg-background/90"
				copyright="chat-room-render"
				links={[
					{ label: "Render", href: "https://render.com" },
					{ label: "Rivet", href: "https://rivet.dev" },
				]}
			/>
		</div>
	);
}
