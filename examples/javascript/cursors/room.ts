import type { Rpc } from "@rivet-gg/actor";
import { throttle } from "@std/async/unstable-throttle";
import { chooseRandomColor } from "./utils.ts";

const ENTITY_COUNT = 10;

interface State {
	/** Entities that can be dragged around. */
	entities: Entity[];
}

interface Entity {
	x: number;
	y: number;
	color: string;
}

interface ConnState {
	x: number;
	y: number;
	color: string;
}

/** State that gets broadcasted to all clients. */
export interface BroadcastState {
	entities: Entity[];
	cursors: { id: number; x: number; y: number; color: string }[];
}

export default class Room extends Actor<State, undefined, ConnState> {
	/** Broadcasts state to all clients. Throttled to 20 broadcasts per second. */
	#broadcastState = throttle(() => {
		const state: BroadcastState = {
			entities: this.state.entities,
			cursors: this.connections
				.values()
				.map((c) => ({
					id: c.id,
					x: c.state.x,
					y: c.state.y,
					color: c.state.color,
				}))
				.toArray(),
		};
		this._broadcast("state", state);
	}, 50);

	override _onInitialize(): State {
		const entities = [];
		for (let i = 0; i < ENTITY_COUNT; i++) {
			entities.push({
				x: Math.random(),
				y: Math.random(),
				color: chooseRandomColor(),
			});
		}
		return { entities };
	}

	override _onBeforeConnect(
		_opts: OnBeforeConnectOpts<undefined>,
	): ConnState {
		return {
			x: Math.random(),
			y: Math.random(),
			color: chooseRandomColor(),
		};
	}

	moveCursor(rpc: Rpc<Room>, x: number, y: number) {
		rpc.connection.state.x = x;
		rpc.connection.state.y = y;
		this.#broadcastState();
	}

	moveEntity(rpc: Rpc<Room>, idx: number, x: number, y: number) {
		this.state.entities[idx].x = x;
		this.state.entities[idx].y = y;
		this.state.entities[idx].color = rpc.connection.state.color;
		this.#broadcastState();
	}
}

