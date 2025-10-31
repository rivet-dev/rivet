import { createRivetKit } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type { Contact } from "../backend/types";
import type { registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

export function App() {
	const [contacts, setContacts] = useState<Contact[]>([]);
	const [name, setName] = useState("");
	const [email, setEmail] = useState("");
	const [phone, setPhone] = useState("");
	const [syncStatus, setSyncStatus] = useState<"Idle" | "Syncing" | "Synced" | "Offline">("Idle");
	const [stats, setStats] = useState({ totalContacts: 0, lastSyncTime: 0, deletedContacts: 0 });

	const lastSyncTime = useRef(0);
	const syncIntervalRef = useRef<NodeJS.Timeout | null>(null);

	const contactsActor = useActor({
		name: "contacts",
		key: ["global"],
	});

	// Load initial contacts and stats
	useEffect(() => {
		if (!contactsActor.connection) return;

		const loadInitialData = async () => {
			try {
				const data = await contactsActor.connection!.getChanges(0);
				setContacts(data.changes);
				lastSyncTime.current = data.timestamp;
				setSyncStatus("Synced");

				const statsData = await contactsActor.connection!.getSyncStats();
				setStats(statsData);
			} catch (error) {
				setSyncStatus("Offline");
			}
		};

		loadInitialData();
	}, [contactsActor.connection]);

	// Handle contact events from other clients
	contactsActor.useEvent("contactsChanged", ({ contacts: updatedContacts }: { contacts: Contact[] }) => {
		setContacts((prev) => {
			const contactMap = new Map(prev.map((c) => [c.id, c]));

			updatedContacts.forEach((contact) => {
				const existing = contactMap.get(contact.id);
				if (!existing || existing.updatedAt < contact.updatedAt) {
					contactMap.set(contact.id, contact);
				}
			});

			return Array.from(contactMap.values()).filter(c => c.name !== "");
		});

		// Update stats when contacts change
		if (contactsActor.connection) {
			contactsActor.connection.getSyncStats().then(setStats);
		}
	});

	// Periodic sync - every 5 seconds
	useEffect(() => {
		if (!contactsActor.connection) return;

		const sync = async () => {
			setSyncStatus("Syncing");

			try {
				// Get remote changes
				const changes = await contactsActor.connection!.getChanges(lastSyncTime.current);

				// Apply remote changes
				if (changes.changes.length > 0) {
					setContacts((prev) => {
						const contactMap = new Map(prev.map((c) => [c.id, c]));

						changes.changes.forEach((contact) => {
							const existing = contactMap.get(contact.id);
							if (!existing || existing.updatedAt < contact.updatedAt) {
								contactMap.set(contact.id, contact);
							}
						});

						return Array.from(contactMap.values()).filter(c => c.name !== "");
					});
				}

				// Push local changes
				const localChanges = contacts.filter(
					(c) => c.updatedAt > lastSyncTime.current,
				);
				if (localChanges.length > 0) {
					await contactsActor.connection!.pushChanges(localChanges);
				}

				lastSyncTime.current = changes.timestamp;
				setSyncStatus("Synced");

				// Update stats
				const statsData = await contactsActor.connection!.getSyncStats();
				setStats(statsData);
			} catch (error) {
				setSyncStatus("Offline");
			}
		};

		syncIntervalRef.current = setInterval(sync, 5000);

		return () => {
			if (syncIntervalRef.current) {
				clearInterval(syncIntervalRef.current);
				syncIntervalRef.current = null;
			}
		};
	}, [contactsActor.connection, contacts]);

	// Add new contact (local first)
	const addContact = async () => {
		if (!name.trim()) return;

		const newContact: Contact = {
			id: Date.now().toString(),
			name,
			email,
			phone,
			updatedAt: Date.now(),
		};

		// Add locally first for immediate UI feedback
		setContacts((prev) => [...prev, newContact]);

		// Then sync to server
		if (contactsActor.connection) {
			try {
				await contactsActor.connection.pushChanges([newContact]);
				const statsData = await contactsActor.connection.getSyncStats();
				setStats(statsData);
			} catch (error) {
				setSyncStatus("Offline");
			}
		}

		setName("");
		setEmail("");
		setPhone("");
	};

	// Delete contact (implemented as update with empty name)
	const deleteContact = async (id: string) => {
		const deletedContact = contacts.find(c => c.id === id);
		if (!deletedContact) return;

		const updatedContact: Contact = {
			...deletedContact,
			name: "", // Mark as deleted
			updatedAt: Date.now()
		};

		// Remove locally first for immediate UI feedback
		setContacts((prev) => prev.filter((c) => c.id !== id));

		// Then sync to server
		if (contactsActor.connection) {
			try {
				await contactsActor.connection.pushChanges([updatedContact]);
				const statsData = await contactsActor.connection.getSyncStats();
				setStats(statsData);
			} catch (error) {
				setSyncStatus("Offline");
			}
		}
	};

	// Manual sync
	const handleSync = async () => {
		if (!contactsActor.connection) return;

		setSyncStatus("Syncing");

		try {
			// Push all contacts
			await contactsActor.connection.pushChanges(contacts);

			// Get all changes
			const changes = await contactsActor.connection.getChanges(0);

			setContacts(changes.changes.filter(c => c.name !== ""));
			lastSyncTime.current = changes.timestamp;
			setSyncStatus("Synced");

			// Update stats
			const statsData = await contactsActor.connection.getSyncStats();
			setStats(statsData);
		} catch (error) {
			setSyncStatus("Offline");
		}
	};

	// Reset all data
	const handleReset = async () => {
		if (!contactsActor.connection) return;

		try {
			await contactsActor.connection.reset();
			setContacts([]);
			lastSyncTime.current = Date.now();
			setSyncStatus("Synced");
			setStats({ totalContacts: 0, lastSyncTime: Date.now(), deletedContacts: 0 });
		} catch (error) {
			setSyncStatus("Offline");
		}
	};

	// Handle form submission
	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		addContact();
	};

	return (
		<div className="app-container">
			<div className="header">
				<h1>Sync Contacts</h1>
				<div className="sync-status">
					<span className={`status-indicator status-${syncStatus.toLowerCase()}`}>
						{syncStatus}
					</span>
					<button 
						className="sync-button" 
						onClick={handleSync}
						disabled={!contactsActor.connection || syncStatus === "Syncing"}
					>
						Sync Now
					</button>
					<button 
						className="sync-button" 
						onClick={handleReset}
						disabled={!contactsActor.connection}
						style={{ backgroundColor: "#dc3545" }}
					>
						Reset
					</button>
				</div>
			</div>

			<div className="info-box">
				<h3>How it works</h3>
				<p>
					This contact sync system demonstrates offline-first synchronization with conflict resolution. 
					Add contacts and they'll sync across all connected clients. The system handles conflicts using 
					"last write wins" based on timestamps, and supports offline operation with automatic sync when reconnected.
				</p>
			</div>

			<div className="add-contact-section">
				<h3>Add New Contact</h3>
				<form onSubmit={handleSubmit} className="contact-form">
					<input
						type="text"
						placeholder="Name *"
						value={name}
						onChange={(e) => setName(e.target.value)}
						required
						disabled={!contactsActor.connection}
					/>
					<input
						type="email"
						placeholder="Email"
						value={email}
						onChange={(e) => setEmail(e.target.value)}
						disabled={!contactsActor.connection}
					/>
					<input
						type="tel"
						placeholder="Phone"
						value={phone}
						onChange={(e) => setPhone(e.target.value)}
						disabled={!contactsActor.connection}
					/>
					<button 
						type="submit" 
						className="add-button"
						disabled={!contactsActor.connection || !name.trim()}
					>
						Add Contact
					</button>
				</form>
			</div>

			<div className="contacts-list">
				<h3>Contacts ({contacts.length})</h3>
				{contacts.length === 0 ? (
					<div className="empty-state">
						No contacts yet. Add some contacts to get started!
					</div>
				) : (
					contacts.map((contact) => (
						<div key={contact.id} className="contact-item">
							<div className="contact-info">
								<div className="contact-name">{contact.name}</div>
								<div className="contact-details">
									{contact.email && (
										<div className="contact-email">📧 {contact.email}</div>
									)}
									{contact.phone && (
										<div className="contact-phone">📞 {contact.phone}</div>
									)}
								</div>
							</div>
							<button
								className="delete-button"
								onClick={() => deleteContact(contact.id)}
								disabled={!contactsActor.connection}
							>
								Delete
							</button>
						</div>
					))
				)}
			</div>

			<div className="stats">
				<div className="stat-item">
					<div className="stat-value">{stats.totalContacts}</div>
					<div className="stat-label">Total Contacts</div>
				</div>
				<div className="stat-item">
					<div className="stat-value">{stats.deletedContacts}</div>
					<div className="stat-label">Deleted Items</div>
				</div>
				<div className="stat-item">
					<div className="stat-value">
						{stats.lastSyncTime ? new Date(stats.lastSyncTime).toLocaleTimeString() : "—"}
					</div>
					<div className="stat-label">Last Sync</div>
				</div>
			</div>
		</div>
	);
}
