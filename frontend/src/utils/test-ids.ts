export const TEST_IDS = {
	Onboarding: {
		PathSelection: "onboarding-path-selection",
		PathSelectionAgent: "onboarding-path-agent",
		PathSelectionTemplate: "onboarding-path-template",
		PathSelectionManual: "onboarding-path-manual",

		TemplateList: "onboarding-template-list",
		TemplateOption: (templateName: string) =>
			`onboarding-template-option-${templateName}`,

		CreateProjectCard: "create-project-card",
		CreateTemplateProjectCard: "create-template-project-card",

		IntegrationProviderSelection:
			"onboarding-integration-provider-selection",

		IntegrationProviderOption: (providerName: string) =>
			`integration-provider-option-${providerName}`,
	},
};
