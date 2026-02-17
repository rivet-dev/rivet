export const TEST_IDS = {
	Onboarding: {
		PathSelection: "onboarding-path-selection",
		PathSelectionAgent: "onboarding-path-agent",
		PathSelectionManual: "onboarding-path-manual",

		CreateProjectCard: "create-project-card",

		IntegrationProviderSelection:
			"onboarding-integration-provider-selection",

		IntegrationProviderOption: (providerName: string) =>
			`integration-provider-option-${providerName}`,
	},
};
