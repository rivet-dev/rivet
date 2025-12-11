import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { client } from "@/lib/client";
import { templates } from "../../shared/types";
import { Button } from "@/components/ui/button";

export default function HomePage() {
	const [prompt, setPrompt] = useState("");
	const [framework, setFramework] = useState("react");
	const [isLoading, setIsLoading] = useState(false);
	const navigate = useNavigate();

	const handleSubmit = async () => {
		setIsLoading(true);
		navigate(`/app/new?message=${encodeURIComponent(prompt)}&template=${framework}`);
	};

	return (
		<main className="min-h-screen p-4 relative">
			<div className="w-full max-w-lg px-4 mx-auto flex flex-col items-center mt-16 sm:mt-24 md:mt-32">
				<p className="text-foreground text-center mb-6 text-3xl sm:text-4xl md:text-5xl font-bold">
					Rivet AI App Builder
				</p>
				<div className="w-full relative my-5">
					<div className="w-full bg-card rounded-md border p-4">
						<div className="flex gap-2 mb-4">
							<select
								value={framework}
								onChange={(e) => setFramework(e.target.value)}
								className="border rounded-md px-3 py-2 bg-background"
							>
								{Object.entries(templates).map(([key, template]) => (
									<option key={key} value={key}>{template.name}</option>
								))}
							</select>
						</div>
						<textarea
							placeholder="What do you want to build?"
							value={prompt}
							onChange={(e) => setPrompt(e.target.value)}
							className="w-full min-h-[100px] p-3 border rounded-md resize-none bg-background"
							onKeyDown={(e) => {
								if (e.key === "Enter" && !e.shiftKey && prompt.trim()) {
									e.preventDefault();
									handleSubmit();
								}
							}}
						/>
						<div className="flex justify-end mt-2">
							<Button onClick={handleSubmit} disabled={isLoading || !prompt.trim()}>
								{isLoading ? "Creating..." : "Start Creating"}
							</Button>
						</div>
					</div>
				</div>
			</div>
			<div className="border-t py-8">
				<UserApps />
			</div>
		</main>
	);
}

function UserApps() {
	const [apps, setApps] = useState<Array<{ id: string; name: string; createdAt: number }>>([]);
	const [isLoading, setIsLoading] = useState(true);

	useEffect(() => {
		loadApps();
	}, []);

	async function loadApps() {
		setIsLoading(true);
		try {
			const userAppList = await client.userAppList.getOrCreate(["global"]);
			const appIds = await userAppList.getAppIds();
			const appInfos: Array<{ id: string; name: string; createdAt: number }> = [];
			for (const appId of appIds) {
				try {
					const info = await client.userApp.get([appId]).getInfo();
					if (info) {
						appInfos.push({ id: info.id, name: info.name, createdAt: info.createdAt });
					}
				} catch {}
			}
			appInfos.sort((a, b) => b.createdAt - a.createdAt);
			setApps(appInfos);
		} catch (err) {
			console.error("Failed to load apps:", err);
		}
		setIsLoading(false);
	}

	if (isLoading) return <div className="px-4 text-muted-foreground">Loading apps...</div>;
	if (apps.length === 0) return <div className="px-4 text-muted-foreground">No apps yet. Create one above!</div>;

	return (
		<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 px-4">
			{apps.map((app) => (
				<a key={app.id} href={`/app/${app.id}`} className="border rounded-lg p-4 hover:border-primary transition-colors">
					<h3 className="font-medium truncate">{app.name}</h3>
					<p className="text-xs text-muted-foreground mt-1">{new Date(app.createdAt).toLocaleDateString()}</p>
				</a>
			))}
		</div>
	);
}
