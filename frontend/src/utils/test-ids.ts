export const TEST_IDS = {
	Onboarding: {
		PathSelection: "onboarding-path-selection",
		PathSelectionAgent: "onboarding-path-agent",
		PathSelectionManual: "onboarding-path-manual",

		CreateProjectCard: "create-project-card",

		GettingStartedWizard: "onboarding-getting-started-wizard",
		StepperSkipToDeploy: "onboarding-stepper-skip-to-deploy",
		VerificationStep: "onboarding-verification-step",
		WaitingForActor: "onboarding-waiting-for-actor",

		IntegrationProviderSelection:
			"onboarding-integration-provider-selection",

		IntegrationProviderOption: (providerName: string) =>
			`integration-provider-option-${providerName}`,
	},

	Engine: {
		AdminTokenForm: "engine-admin-token-modal",
	},

	Layout: {
		Sidebar: "layout-sidebar",
		Main: "layout-main",
	},

	Actors: {
		DetailsPanel: "actors-details-panel",
	},
};
