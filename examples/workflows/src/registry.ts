import { actor, setup, UserError } from "rivetkit";
import { assertUnreachable } from "rivetkit/utils";
import type { Order, OrderInput } from "./types";

const orderWorkflow = actor({
	createState: (c, input): Order => ({
		id: c.actorId,
		customer: (input as OrderInput).customer,
		state: "pending",
		createdAt: Date.now(),
		updatedAt: Date.now(),
	}),
	actions: {
		advance: (c) => {
			switch (c.state.state) {
				case "pending":
					// Execute durable operation here
					c.state.state = "packed";
					break;
				case "packed":
					// Execute durable operation here
					c.state.state = "shipped";
					break;
				case "shipped":
					// Execute durable operation here
					c.state.state = "delivered";
					break;
				case "delivered":
					// Execute durable operation here
					throw new UserError("order already delivered", {
						code: "order_complete",
					});
				default:
					assertUnreachable(c.state.state);
			}

			c.state.updatedAt = Date.now();

			c.broadcast("orderAdvanced", c.state);

			return c.state;
		},

		getOrder: (c) => c.state,
	},
});

export const registry = setup({
	use: { orderWorkflow },
});

export type Registry = typeof registry;
