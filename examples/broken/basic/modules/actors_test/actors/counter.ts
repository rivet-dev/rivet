import { ActorBase, ActorContext, Empty } from "../module.gen.ts";

interface Input {
}

interface State {
	count: number;
	lastTick: number;
}

export interface FetchResponse {
	count: number;
	lastTick: number;
	schedule: any;
}

export const TICK_INTERVAL = 200;

export class Actor extends ActorBase<Input, State> {
	public initialize(_input: Input): State {
		this.schedule.after(TICK_INTERVAL, "tick", undefined);
		return { count: 0, lastTick: 0 };
	}

	async rpcFetchCount(_ctx: ActorContext, _req: Empty): Promise<FetchResponse> {
		return {
			count: this.state.count,
			lastTick: this.state.lastTick,
			schedule: await this.schedule.__inspect(),
		};
	}

	private tick(ctx: ActorContext) {
		this.schedule.after(TICK_INTERVAL, "tick", undefined);
		this.state.count += 1;
		this.state.lastTick = Date.now();
	}
}
