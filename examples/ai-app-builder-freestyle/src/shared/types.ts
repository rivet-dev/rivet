// Shared types and constants that can be used in both frontend and backend

export interface AppInfo {
	id: string;
	name: string;
	createdAt: number;
	gitRepo?: string;
	previewDomain?: string | null;
}

export interface AppDeployment {
	deploymentId: string;
	commit: string;
	createdAt: number;
}

export interface UIMessage {
	id: string;
	role: "user" | "assistant" | "system";
	parts: Array<{ type: string; text?: string; [key: string]: unknown }>;
}

// Templates
export const templates: Record<string, { name: string; repo: string; logo: string }> = {
	react: {
		name: "React",
		repo: "https://github.com/rivet-dev/template-freestyle",
		logo: "/logos/react.svg",
	},
};
