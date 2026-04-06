export interface BuildingType {
	id: string;
	name: string;
	cost: number;
	productionRate: number;
	productionIntervalMs: number;
}

export const BUILDINGS: BuildingType[] = [
	{ id: "farm", name: "Farm", cost: 0, productionRate: 5, productionIntervalMs: 10_000 },
	{ id: "mine", name: "Mine", cost: 50, productionRate: 15, productionIntervalMs: 30_000 },
	{ id: "factory", name: "Factory", cost: 200, productionRate: 50, productionIntervalMs: 60_000 },
	{ id: "lab", name: "Lab", cost: 1000, productionRate: 200, productionIntervalMs: 120_000 },
];

export const STARTING_RESOURCES = 10;
