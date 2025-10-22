import { actor, setup } from "rivetkit";
import type { Contact } from "./types";

const contacts = actor({
	// State is automatically persisted
	// Persistent state that survives restarts
	state: {
		contacts: {} as Record<string, Contact>,
		lastSyncTime: Date.now(),
	},

	actions: {
		// Callable functions from clients
		getChanges: (c, after = 0) => {
			const changes = Object.values(c.state.contacts).filter(
				(contact) => contact.updatedAt > after,
			);

			return {
				changes,
				timestamp: Date.now(),
			};
		},

		pushChanges: (c, contactList: Contact[]) => {
			let changed = false;

			contactList.forEach((contact) => {
				const existing = c.state.contacts[contact.id];

				// Last write wins conflict resolution based on timestamp
				if (!existing || existing.updatedAt < contact.updatedAt) {
					// State changes are automatically persisted
					c.state.contacts[contact.id] = contact;
					changed = true;
				}
			});

			// Update last sync time
			c.state.lastSyncTime = Date.now();

			if (changed) {
				// Send events to all connected clients
				c.broadcast("contactsChanged", {
					contacts: Object.values(c.state.contacts).filter(
						(c) => c.name !== "",
					),
				});
			}

			return { timestamp: c.state.lastSyncTime };
		},

		getAllContacts: (c) => {
			return Object.values(c.state.contacts).filter(
				(contact) => contact.name !== "",
			);
		},

		getSyncStats: (c) => {
			const allContacts = Object.values(c.state.contacts);
			const activeContacts = allContacts.filter(
				(contact) => contact.name !== "",
			);

			return {
				totalContacts: activeContacts.length,
				lastSyncTime: c.state.lastSyncTime,
				deletedContacts: allContacts.filter(
					(contact) => contact.name === "",
				).length,
			};
		},

		reset: (c) => {
			c.state.contacts = {};
			c.state.lastSyncTime = Date.now();

			c.broadcast("contactsChanged", {
				contacts: [],
			});

			return { timestamp: c.state.lastSyncTime };
		},
	},
});

// Register actors for use
export const registry = setup({
	use: { contacts },
});
