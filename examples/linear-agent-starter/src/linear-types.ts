// These types need to be manually defined since @linear/sdk doesn't include types from webhooks

export interface LinearWebhookEvent {
	type: string;
	action: string;
	createdAt: string;
	organizationId: string;
	oauthClientId: string;
	appUserId: string;
	notification: WebhookNotification;
	webhookTimestamp: number;
	webhookId: string;
}

export interface WebhookNotification {
	id: string;
	createdAt: string;
	updatedAt: string;
	archivedAt: string | null;
	type: string;
	actorId: string;
	externalUserActorId: string | null;
	userId: string;
	readAt: string | null;
	emailedAt: string | null;
	snoozedUntilAt: string | null;
	unsnoozedAt: string | null;
	issueId: string;
	issue: WebhookIssue;
	commentId?: string;
	comment?: WebhookComment;
	actor: WebhookActor;
	reactionEmoji?: string;
}

export interface WebhookIssue {
	id: string;
	title: string;
	teamId: string;
	team: unknown;
	identifier: string;
	url: string;
}

export interface WebhookComment {
	id: string;
	body: string;
	userId: string;
	issueId: string;
}

export interface WebhookActor {
	id: string;
	name: string;
	email: string;
	avatarUrl: string;
	url: string;
}
