import {
	Actor,
	type Connection,
	type OnBeforeConnectOptions,
	type Rpc,
	UserError,
} from "@rivet-gg/actor";
import { validateUsername } from "./utils.ts";

interface ConnParams {
	username: string;
}

interface ConnState {
	username: string;
	typing: boolean;
	lastSentMessage: number;
}

interface ChatMessage {
	id: string;
	sentAt: number;
	username: string;
	text: string;
}

export default class ChatRoom extends Actor<undefined, ConnParams, ConnState> {
	override _onBeforeConnect(
		opts: OnBeforeConnectOptions<ChatRoom>,
	): ConnState {
		const username = opts.parameters.username;
		validateUsername(username);
		return {
			username: opts.parameters.username,
			typing: false,
			lastSentMessage: 0,
		};
	}

	protected override _onConnect(
		connection: Connection<ChatRoom>,
	): void | Promise<void> {
		this.#broadcastPresence();
	}

	protected override _onDisconnect(
		connection: Connection<ChatRoom>,
	): void | Promise<void> {
		this.#broadcastPresence();
		this.#broadcastTyping();
	}

	#broadcastPresence() {
		const connectedUsers = this._connections.values().map((c) => ({
			username: c.state.username,
			typing: c.state.typing,
		}));
		this._broadcast("presenceUpdate", connectedUsers);
	}

	setTyping(rpc: Rpc<ChatRoom>, typing: boolean) {
		rpc.connection.state.typing = typing;
		this.#broadcastPresence();
	}

	async sendMessage(rpc: Rpc<ChatRoom>, text: string) {
		// Rate limit messages
		if (Date.now() - rpc.connection.state.lastSentMessage < 500) {
			throw new UserError("Sending messages too fast");
		}

		const message: ChatMessage = {
			id: crypto.randomUUID(),
			sentAt: Date.now(),
			username: rpc.connection.state.username,
			text,
		};
		rpc.connection.state.lastSentMessage = message.sentAt;

		await this._kv.put(["messages", message.sentAt, message.id], message);

		this._broadcast("newMessage", message);
	}

	async fetchHistory(
		rpc: Rpc<ChatRoom>,
		after?: number,
	): Promise<ChatMessage[]> {
		// List messages
		let messages: ChatMessage[];
		if (after) {
			messages = await this._kv.list<ChatMessage>({
				prefix: ["messages"],
				after: ["messages", after],
			});
		} else {
			messages = await this._kv.list<ChatMessage>({
				prefix: ["messages"],
			});
		}

		return messages;
	}
}
