export interface Config {
	rivetEndpoint: string;
	rivetNamespace: string;
	vus: number;
	duration: string;
	rampUpDuration: string;
	disableHealthcheck?: boolean;
	disableSleep?: boolean;
}

export interface Actor {
	actor_id: string;
}

export interface CreateActorResponse {
	actor: Actor;
}
