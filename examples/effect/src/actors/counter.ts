import { actor } from "rivetkit";
import { Action } from "../effect";

export const counter = actor({
	state: {
		count: 0,
	},
	actions: {
		increment: Action.effect(function* (c, x: number) {
			yield* Action.updateState(c, (s) => {
				s.count += x;
			});
			const s = yield* Action.state(c);
			yield* Action.broadcast(c, "newCount", s.count);
			return s.count;
		}),
		getCount: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return s.count;
		}),
		// increment: Action.createWithSchema(
		// 	IncrementInput,
		// 	function* (c, { amount }) {
		// 		yield* Actor.updateState(c, (s) => {
		// 			s.count += amount;
		// 		});
		// 		const s = yield* Actor.state(c);
		// 		return s.count;
		// 	},
		// ),
	},
});
