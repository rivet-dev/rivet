export type CampaignInput = {
	email: string;
};

export type CampaignState = CampaignInput & {
	lastSentAt?: number;
	lastMessageId?: string;
	nextSendAt?: number;
};
