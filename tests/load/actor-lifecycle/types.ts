export interface Config {
	rivetEndpoint: string;
	rivetNamepace: string;
	vus: number;
	duration: string;
	rampUpDuration: string;
	disableHealthcheck?: boolean;
	disableSleep?: boolean;
}

export interface Actor {
	actor_id: string;
	addresses_http: {
		main: {
			hostname: string;
			port: number;
		};
	};
}

export interface CreateActorResponse {
	actor: Actor;
}
