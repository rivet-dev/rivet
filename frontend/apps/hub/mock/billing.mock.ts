import { defineMock } from "vite-plugin-mock-dev-server";

export default defineMock([
	{
		url: "/api/cloud/bootstrap",
		body: {
			// use "enterprise" to remove billing
			cluster: "oss",
			access: "public",
			domains: {},
			deploy_hash: "0",
		},
	},
	{
		url: "/api/cloud/games/:id/billing",
		body: {
			active_plan: "indie",
			// change to different plan, to show downgrade/upgrade warning
			plan: "indie",
			subscription: {
				period_start_ts: "2024-06-06T18:27:45.601Z",
				period_end_ts: "2024-07-06T18:27:45.601Z",
			},
			watch: { index: "0000" },
		},
	},
	{
		url: "/api/cloud/groups/:id/billing/usage",
		body: {
			games: [],
		},
	},
	{
		url: "/api/cloud/groups/:id/billing",
		body: {
			group: {
				payment_method_attached_ts: "2024-06-06T18:27:45.601Z",
				payment_method_valid_ts: "2024-06-06T18:27:45.601Z",
				// show info about missing payment method
				// payment_method_attached_ts: null,
				// payment_method_valid_ts: null
			},
			watch: { index: "0000" },
		},
	},
	{
		url: "/api/cloud/groups/:id/billing/stripe-portal-session",
		method: "POST",
		body: {
			stripe_session_url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
		},
	},
]);
