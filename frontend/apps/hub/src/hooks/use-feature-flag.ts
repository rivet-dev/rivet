import { useFeatureFlagEnabled } from "posthog-js/react";

export type FeatureFlag =
	| "hub-opengb-backend"
	| "hub-upgraded-alert"
	| (string & {});

export const useFeatureFlag = (flag: FeatureFlag) => {
	return useFeatureFlagEnabled(flag);
};
