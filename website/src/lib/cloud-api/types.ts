// Cloud API types for autofill functionality

export interface CloudBootstrapResponse {
	user: CloudUser | null;
	projects: CloudProject[];
}

export interface CloudUser {
	id: string;
	username: string;
}

export interface CloudProject {
	id: string;
	slug: string;
	name: string;
	organizationId: string;
	organizationSlug: string;
}

export interface CloudNamespace {
	id: string;
	slug: string;
	name: string;
	projectId: string;
}

export interface CloudNamespaceToken {
	token: string;
	engineUrl: string;
}
