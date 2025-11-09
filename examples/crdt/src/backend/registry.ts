import { actor, setup } from "rivetkit";
import * as Y from "yjs";
import { applyUpdate, encodeStateAsUpdate } from "yjs";

export const yjsDocument = actor({
	// Persistent state that survives restarts.
	state: {
		docData: new Uint8Array(), // Raw Yjs document snapshot
		lastModified: 0,
	},

	createVars: () => ({
		doc: new Y.Doc(),
	}),

	onWake: (c) => {
		if (c.state.docData.length > 0) {
			applyUpdate(c.vars.doc, c.state.docData);
		}
	},

	// Handle client connections.
	onConnect: (c, conn) => {
		const update = encodeStateAsUpdate(c.vars.doc);
		conn.send("initialState", { update });
	},

	actions: {
		// Callable functions from clients.
		applyUpdate: (c, update: Uint8Array) => {
			applyUpdate(c.vars.doc, update);

			const fullState = encodeStateAsUpdate(
				c.vars.doc,
			) as Uint8Array<ArrayBuffer>;
			// State changes are automatically persisted
			c.state.docData = fullState;
			c.state.lastModified = Date.now();

			// Send events to all connected clients.
			c.broadcast("update", { update });
		},

		getState: (c) => ({
			docData: c.state.docData,
			lastModified: c.state.lastModified,
		}),
	},
});

// Register actors for use.
export const registry = setup({
	use: { yjsDocument },
});
