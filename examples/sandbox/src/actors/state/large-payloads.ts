import { actor } from "rivetkit";

/**
 * Actor for testing large payloads without connections
 */
export const largePayloadActor = actor({
	state: {},
	actions: {
		/**
		 * Accepts a large request payload and returns its size
		 */
		processLargeRequest: (c, data: { items: string[] }) => {
			return {
				itemCount: data.items.length,
				firstItem: data.items[0],
				lastItem: data.items[data.items.length - 1],
			};
		},

		/**
		 * Returns a large response payload
		 */
		getLargeResponse: (c, itemCount: number) => {
			const items: string[] = [];
			for (let i = 0; i < itemCount; i++) {
				items.push(`Item ${i} with some additional text to increase size`);
			}
			return { items };
		},

		/**
		 * Echo back the request data
		 */
		echo: (c, data: unknown) => {
			return data;
		},
	},
});

/**
 * Actor for testing large payloads with connections
 */
export const largePayloadConnActor = actor({
	state: {},
	connState: {
		lastRequestSize: 0,
	},
	actions: {
		/**
		 * Accepts a large request payload and returns its size
		 */
		processLargeRequest: (c, data: { items: string[] }) => {
			c.conn.state.lastRequestSize = data.items.length;
			return {
				itemCount: data.items.length,
				firstItem: data.items[0],
				lastItem: data.items[data.items.length - 1],
			};
		},

		/**
		 * Returns a large response payload
		 */
		getLargeResponse: (c, itemCount: number) => {
			const items: string[] = [];
			for (let i = 0; i < itemCount; i++) {
				items.push(`Item ${i} with some additional text to increase size`);
			}
			return { items };
		},

		/**
		 * Echo back the request data
		 */
		echo: (c, data: unknown) => {
			return data;
		},

		/**
		 * Get the last request size
		 */
		getLastRequestSize: (c) => {
			return c.conn.state.lastRequestSize;
		},
	},
});
