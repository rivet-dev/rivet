export interface Datacenter {
	name: string;
	id: number; // u16
	peer_id: number; // u64
	engines: number;
	runners: number;
}

export interface TemplateConfig {
	networkMode: "bridge" | "host";
	datacenters: Datacenter[];
}

// TODO: Add support for splitting up engine/guard services
export const TEMPLATES: Record<string, TemplateConfig> = {
	dev: {
		networkMode: "bridge",
		datacenters: [
			{
				name: "local",
				id: 1,
				peer_id: 1,
				engines: 1,
				runners: 1,
			},
		],
	},
	"dev-multinode": {
		networkMode: "bridge",
		datacenters: [
			{
				name: "local",
				id: 1,
				peer_id: 1,
				engines: 3,
				runners: 3,
			},
		],
	},
	"dev-multidc": {
		networkMode: "bridge",
		datacenters: [
			{
				name: "dc-a",
				id: 1,
				peer_id: 1,
				engines: 1,
				runners: 1,
			},
			{
				name: "dc-b",
				id: 2,
				peer_id: 2,
				engines: 1,
				runners: 1,
			},
			{
				name: "dc-c",
				id: 3,
				peer_id: 3,
				engines: 1,
				runners: 1,
			},
		],
	},
	"dev-multidc-multinode": {
		networkMode: "bridge",
		datacenters: [
			{
				name: "dc-a",
				id: 1,
				peer_id: 1,
				engines: 3,
				runners: 3,
			},
			{
				name: "dc-b",
				id: 2,
				peer_id: 2,
				engines: 3,
				runners: 3,
			},
			{
				name: "dc-c",
				id: 3,
				peer_id: 3,
				engines: 3,
				runners: 3,
			},
		],
	},
	"dev-host": {
		networkMode: "host",
		datacenters: [
			{
				name: "local",
				id: 1,
				peer_id: 1,
				engines: 1,
				runners: 1,
			},
		],
	},
};
