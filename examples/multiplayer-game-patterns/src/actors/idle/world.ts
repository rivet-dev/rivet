import { type ActorContextOf, actor, event } from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../index.ts";
import { BUILDINGS, type BuildingType, STARTING_RESOURCES } from "./config.ts";

// The world columns that the leaderboard ranks.
type ScoreRow = {
	player_id: string;
	player_name: string;
	total_produced: number;
};

interface IdleSnapshot {
	playerId: string;
	playerName: string;
	resources: number;
	totalProduced: number;
	buildings: Array<{
		id: string;
		typeId: string;
		name: string;
		productionRate: number;
		productionIntervalMs: number;
		builtAt: number;
		lastCollectedAt: number;
	}>;
}

export const idleWorld = actor({
	options: { name: "Idle - World", icon: "industry" },
	events: {
		stateUpdate: event<IdleSnapshot>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS world (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					player_id TEXT NOT NULL,
					player_name TEXT NOT NULL,
					resources INTEGER NOT NULL,
					total_produced INTEGER NOT NULL,
					initialized INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS buildings (
					id TEXT PRIMARY KEY,
					type_id TEXT NOT NULL,
					built_at INTEGER NOT NULL,
					last_collected_at INTEGER NOT NULL
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO world (id, player_id, player_name, resources, total_produced, initialized) VALUES (1, '', '', ?, 0, 0)",
				STARTING_RESOURCES,
			);
		},
	}),
	actions: {
		// docs:start build
		initialize: async (c, input: { playerName: string; playerId?: string }) => {
			const keyPlayerId = Array.isArray(c.key) ? c.key[0] : c.key;
			const playerId = input.playerId || keyPlayerId || "";
			const farmType = BUILDINGS.find((b) => b.id === "farm")!;
			const farmId = crypto.randomUUID();
			const now = Date.now();

			// Set the initialized flag and add the free farm in one transaction,
			// so only the first call creates a farm.
			const world = await c.db.transaction(
				async (tx) => {
					const [claimed] = await tx.execute<ScoreRow>(
						"UPDATE world SET player_id = ?, player_name = ?, initialized = 1 WHERE id = 1 AND initialized = 0 RETURNING player_id, player_name, total_produced",
						playerId,
						input.playerName,
					);
					if (!claimed) return null;

					await tx.execute(
						"INSERT INTO buildings (id, type_id, built_at, last_collected_at) VALUES (?, ?, ?, ?)",
						farmId,
						farmType.id,
						now,
						now,
					);
					return claimed;
				},
				{ name: "initialize-world" },
			);
			if (!world) {
				await broadcastState(c);
				return;
			}

			await scheduleCollection(c, farmId, farmType.productionIntervalMs);
			updateLeaderboard(c, world);
			await broadcastState(c);
		},
		build: async (c, input: { buildingTypeId: string }) => {
			const buildingType = BUILDINGS.find(
				(b: BuildingType) => b.id === input.buildingTypeId,
			);
			if (!buildingType) {
				throw new Error("Unknown building type");
			}

			const buildingId = crypto.randomUUID();
			const now = Date.now();

			// Spend the cost and add the building in one transaction. The UPDATE
			// only matches when the player can afford the building, so two builds
			// at once cannot spend the same resources.
			await c.db.transaction(
				async (tx) => {
					const [paid] = await tx.execute(
						"UPDATE world SET resources = resources - ? WHERE id = 1 AND resources >= ? RETURNING resources",
						buildingType.cost,
						buildingType.cost,
					);
					if (!paid) {
						throw new Error("Not enough resources");
					}

					await tx.execute(
						"INSERT INTO buildings (id, type_id, built_at, last_collected_at) VALUES (?, ?, ?, ?)",
						buildingId,
						buildingType.id,
						now,
						now,
					);
				},
				{ name: "add-building" },
			);

			await scheduleCollection(
				c,
				buildingId,
				buildingType.productionIntervalMs,
			);
			await broadcastState(c);
		},
		// docs:end build
		// docs:start collect
		collectProduction: async (c, input: { buildingId: string }) => {
			// Read the building and pay out in one transaction, so two runs at
			// once cannot both pay for the same intervals.
			const world = await c.db.transaction(
				async (tx) => {
					const [building] = await tx.execute<{
						type_id: string;
						last_collected_at: number;
					}>(
						"SELECT type_id, last_collected_at FROM buildings WHERE id = ?",
						input.buildingId,
					);
					if (!building) return null;

					const buildingType = BUILDINGS.find(
						(b: BuildingType) => b.id === building.type_id,
					);
					if (!buildingType) return null;

					const now = Date.now();
					const elapsed = now - building.last_collected_at;
					const intervals = Math.floor(
						elapsed / buildingType.productionIntervalMs,
					);
					if (intervals <= 0) return null;

					const produced = intervals * buildingType.productionRate;
					await tx.execute(
						"UPDATE buildings SET last_collected_at = ? WHERE id = ?",
						now,
						input.buildingId,
					);
					const [updated] = await tx.execute<ScoreRow>(
						"UPDATE world SET resources = resources + ?, total_produced = total_produced + ? WHERE id = 1 RETURNING player_id, player_name, total_produced",
						produced,
						produced,
					);
					return updated;
				},
				{ name: "collect-production" },
			);
			if (!world) return;

			updateLeaderboard(c, world);
			await broadcastState(c);
		},
		// docs:end collect
		getState: (c): Promise<IdleSnapshot> => buildSnapshot(c),
		getLeaderboard: async (c) => {
			const client = c.client<typeof registry>();
			return await client.idleLeaderboard
				.getOrCreate(["main"])
				.getTopScores({ limit: 10 });
		},
	},
});

// docs:start schedule
async function scheduleCollection(
	c: ActorContextOf<typeof idleWorld>,
	buildingId: string,
	intervalMs: number,
): Promise<void> {
	await c.cron.every({
		name: `collect-production:${buildingId}`,
		interval: intervalMs,
		action: "collectProduction",
		args: [{ buildingId }],
		maxHistory: 0,
	});
}
// docs:end schedule

// docs:start leaderboard
function updateLeaderboard(
	c: ActorContextOf<typeof idleWorld>,
	world: ScoreRow,
) {
	// Do not wait for the leaderboard, so a slow leaderboard never delays a
	// payout. A failed report is fixed by the next payout's total.
	const client = c.client<typeof registry>();
	client.idleLeaderboard
		.getOrCreate(["main"])
		.updateScore({
			playerId: world.player_id,
			playerName: world.player_name,
			totalProduced: world.total_produced,
		})
		.catch((error) => {
			c.log.warn({ msg: "leaderboard update failed", error });
		});
}
// docs:end leaderboard

async function buildSnapshot(
	c: ActorContextOf<typeof idleWorld>,
): Promise<IdleSnapshot> {
	const [world] = await c.db.execute<{
		player_id: string;
		player_name: string;
		resources: number;
		total_produced: number;
	}>(
		"SELECT player_id, player_name, resources, total_produced FROM world WHERE id = 1",
	);
	const buildings = await c.db.execute<{
		id: string;
		type_id: string;
		built_at: number;
		last_collected_at: number;
	}>(
		"SELECT id, type_id, built_at, last_collected_at FROM buildings ORDER BY built_at",
	);
	return {
		playerId: world!.player_id,
		playerName: world!.player_name,
		resources: world!.resources,
		totalProduced: world!.total_produced,
		buildings: buildings.map((b) => {
			const bType = BUILDINGS.find(
				(bt: BuildingType) => bt.id === b.type_id,
			) as BuildingType;
			return {
				id: b.id,
				typeId: b.type_id,
				name: bType.name,
				productionRate: bType.productionRate,
				productionIntervalMs: bType.productionIntervalMs,
				builtAt: b.built_at,
				lastCollectedAt: b.last_collected_at,
			};
		}),
	};
}

async function broadcastState(c: ActorContextOf<typeof idleWorld>) {
	c.broadcast("stateUpdate", await buildSnapshot(c));
}
