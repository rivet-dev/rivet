export type WorkspaceInput = {
	workspaceId: string;
	channelId: string;
};

export type WorkspaceState = WorkspaceInput & {
	messageCount: number;
	nextReportAt?: number;
};
