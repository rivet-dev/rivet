import { actor, type ActorContextOf, event } from "rivetkit";
import { registry } from "../index.ts";
import { BUILDINGS, STARTING_RESOURCES, type BuildingType } from "./config.ts";

interface BuildingEntry {
	id: string;
	typeId: string;
	builtAt: number;
	lastCollectedAt: number;
}

interface State {
	playerId: string;
	playerName: string;
	resources: number;
	buildings: BuildingEntry[];
	totalProduced: number;
	initialized: boolean;
}

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
	state: {
		playerId: "",
		playerName: "",
		resources: STARTING_RESOURCES,
		buildings: [] as BuildingEntry[],
		totalProduced: 0,
		initialized: false as boolean,
	} satisfies State,
	actions: {
		initialize: (c, input: { playerName: string; playerId?: string }) => {
			if (c.state.initialized) {
				broadcastState(c);
				return;
			}
			c.state.playerName = input.playerName;
			const keyPlayerId = Array.isArray(c.key) ? c.key[0] : c.key;
			if (input.playerId) {
				c.state.playerId = input.playerId;
			} else if (typeof keyPlayerId === "string" && keyPlayerId) {
				c.state.playerId = keyPlayerId;
			}
			c.state.initialized = true;

			const farmType = BUILDINGS.find((b) => b.id === "farm")!;
			const building: BuildingEntry = {
				id: crypto.randomUUID(),
				typeId: farmType.id,
				builtAt: Date.now(),
				lastCollectedAt: Date.now(),
			};
			c.state.buildings.push(building);
			scheduleCollection(c, building.id, farmType.productionIntervalMs);
			updateLeaderboard(c);
			broadcastState(c);
		},
		build: (c, input: { buildingTypeId: string }) => {
			const buildingType = BUILDINGS.find((b: BuildingType) => b.id === input.buildingTypeId);
			if (!buildingType) {
				throw new Error("Unknown building type");
			}
			if (c.state.resources < buildingType.cost) {
				throw new Error("Not enough resources");
			}

			c.state.resources -= buildingType.cost;
			const building: BuildingEntry = {
				id: crypto.randomUUID(),
				typeId: buildingType.id,
				builtAt: Date.now(),
				lastCollectedAt: Date.now(),
			};
			c.state.buildings.push(building);
			scheduleCollection(c, building.id, buildingType.productionIntervalMs);
			broadcastState(c);
		},
		collectProduction: (c, input: { buildingId: string }) => {
			const building = c.state.buildings.find((b: BuildingEntry) => b.id === input.buildingId);
			if (!building) return;

			const buildingType = BUILDINGS.find((b: BuildingType) => b.id === building.typeId);
			if (!buildingType) return;

			const now = Date.now();
			const elapsed = now - building.lastCollectedAt;
			const intervals = Math.floor(elapsed / buildingType.productionIntervalMs);
			if (intervals <= 0) {
				const remaining = buildingType.productionIntervalMs - elapsed;
				scheduleCollection(c, building.id, remaining);
				return;
			}

			const produced = intervals * buildingType.productionRate;
			c.state.resources += produced;
			c.state.totalProduced += produced;
			building.lastCollectedAt = now;

			scheduleCollection(c, building.id, buildingType.productionIntervalMs);

			updateLeaderboard(c);
			broadcastState(c);
		},
		getState: (c): IdleSnapshot => buildSnapshot(c),
		getLeaderboard: async (c) => {
			const client = c.client<typeof registry>();
			return await client.idleLeaderboard
				.getOrCreate(["main"])
				.getTopScores({ limit: 10 });
		},
	},
});

function scheduleCollection(
	c: ActorContextOf<typeof idleWorld>,
	buildingId: string,
	delayMs: number,
) {
	c.schedule.after(delayMs, "collectProduction", { buildingId });
}

function updateLeaderboard(c: ActorContextOf<typeof idleWorld>) {
	const client = c.client<typeof registry>();
	client.idleLeaderboard
		.getOrCreate(["main"])
		.send("updateScore", {
			playerId: c.state.playerId,
			playerName: c.state.playerName,
			totalProduced: c.state.totalProduced,
		})
		.catch(() => {});
}

function buildSnapshot(c: ActorContextOf<typeof idleWorld>): IdleSnapshot {
	return {
		playerId: c.state.playerId,
		playerName: c.state.playerName,
		resources: c.state.resources,
		totalProduced: c.state.totalProduced,
		buildings: c.state.buildings.map((b: BuildingEntry) => {
			const bType = BUILDINGS.find((bt: BuildingType) => bt.id === b.typeId) as BuildingType;
			return {
				id: b.id,
				typeId: b.typeId,
				name: bType.name,
				productionRate: bType.productionRate,
				productionIntervalMs: bType.productionIntervalMs,
				builtAt: b.builtAt,
				lastCollectedAt: b.lastCollectedAt,
			};
		}),
	};
}

function broadcastState(c: ActorContextOf<typeof idleWorld>) {
	c.broadcast("stateUpdate", buildSnapshot(c));
}
