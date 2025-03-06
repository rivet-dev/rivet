export interface Config {
	rivetEndpoint: string;
	rivetServiceToken?: string;
	rivetProject: string;
	rivetEnvironment: string;
	buildName: string;
	region?: string;
	vus: number;
	duration: string;
	rampUpDuration: string;
}

export interface Actor {
	id: string;
	network: {
		ports: {
			http: {
				url: string;
				protocol: string;
				hostname: string;
				port: number;
				path?: string;
			};
		};
	};
}

export interface CreateActorResponse {
	actor: Actor;
}
