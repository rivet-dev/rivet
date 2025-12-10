import { useEffect, useState, useRef } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { client } from "@/lib/client";
import { templates } from "../../shared/types";
import type { UIMessage } from "../../shared/types";

export default function NewAppPage() {
	const navigate = useNavigate();
	const [searchParams] = useSearchParams();
	const [isCreating, setIsCreating] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const hasStartedRef = useRef(false);

	const message = searchParams.get("message") || "";
	const templateId = searchParams.get("template") || "react";

	useEffect(() => {
		if (!hasStartedRef.current) {
			hasStartedRef.current = true;
			createApp();
		}
	}, []);

	async function createApp() {
		if (isCreating) return;
		setIsCreating(true);

		try {
			const template = templates[templateId];
			if (!template) {
				throw new Error(`Template ${templateId} not found`);
			}

			const appId = crypto.randomUUID();
			const appName = message ? decodeURIComponent(message).slice(0, 50) : "New App";

			// Create the app via userAppList (creates git repo and initializes userApp)
			// Note: devServer config is passed at requestDevServer time, not at creation
			const result = await client.userAppList.getOrCreate(["global"]).createApp({
				appId,
				name: appName,
				templateUrl: template.repo,
				templateId,
			});
			const gitRepo = result.gitRepo;

			// Add the initial user message if there is one
			if (message) {
				const userMessage: UIMessage = {
					id: crypto.randomUUID(),
					role: "user",
					parts: [{ type: "text", text: decodeURIComponent(message) }],
				};
				await client.userApp.get([appId]).addMessage(userMessage);

				// Navigate with state indicating there's a pending message to process
				navigate(`/app/${appId}`, { state: { pendingMessage: userMessage, gitRepo } });
				return;
			}

			navigate(`/app/${appId}`);
		} catch (err) {
			console.error("Failed to create app:", err);
			setError(err instanceof Error ? err.message : "Failed to create app");
		}
	}

	if (error) {
		return (
			<div className="flex flex-col items-center justify-center min-h-screen p-4">
				<div className="text-red-500 mb-4">Error: {error}</div>
				<button onClick={() => navigate("/")} className="px-4 py-2 bg-primary text-primary-foreground rounded-md">
					Go back home
				</button>
			</div>
		);
	}

	return (
		<div className="flex flex-col items-center justify-center min-h-screen p-4">
			<div className="animate-pulse">
				<div className="text-lg font-medium">Creating your app...</div>
				<div className="text-sm text-muted-foreground mt-2">
					Setting up {templates[templateId]?.name || templateId} template
				</div>
			</div>
		</div>
	);
}
