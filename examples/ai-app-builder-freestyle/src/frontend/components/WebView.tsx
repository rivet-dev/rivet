import { useRef } from "react";
import {
	FreestyleDevServer,
	FreestyleDevServerHandle,
} from "freestyle-sandboxes/react/dev-server";
import { Button } from "./ui/button";
import { RefreshCwIcon, TerminalIcon, ExternalLinkIcon } from "lucide-react";
import { ShareButton } from "./ShareButton";
import { client } from "@/lib/client";
import "./loader.css";

interface WebViewProps {
	repoId: string;
	appId: string;
	codeServerUrl?: string;
	consoleUrl?: string;
	vmUrl?: string;
}

export default function WebView({ repoId, appId, codeServerUrl, consoleUrl, vmUrl }: WebViewProps) {
	const devServerRef = useRef<FreestyleDevServerHandle>(null);

	async function requestDevServer({ repoId }: { repoId: string }) {
		const result = await client.userApp.get([appId]).requestDevServer();
		return result;
	}

	const inspectorUrl = vmUrl
		? `https://inspect.rivet.dev?t=freestyle&u=${encodeURIComponent(`${vmUrl}/rivet`)}`
		: null;

	return (
		<div className="flex flex-col overflow-hidden h-full border-l transition-opacity duration-700 bg-white">
			<div className="h-12 border-b items-center flex px-2 bg-background sticky top-0 justify-end gap-2">
				{inspectorUrl && (
					<Button
						variant="ghost"
						size="sm"
						className="gap-1.5 cursor-pointer"
						onClick={() => window.open(inspectorUrl, "_blank")}
					>
						<ExternalLinkIcon className="h-4 w-4" />
						Rivet Inspector
					</Button>
				)}
				{codeServerUrl && (
					<Button
						variant="ghost"
						size="sm"
						className="gap-1.5 cursor-pointer"
						onClick={() => window.open(codeServerUrl, "_blank")}
					>
						<img
							src="/logos/vscode.svg"
							className="h-4 w-4"
							alt="VS Code"
						/>
						VS Code
					</Button>
				)}
				{consoleUrl && (
					<Button
						variant="ghost"
						size="sm"
						className="gap-1.5 cursor-pointer"
						onClick={() => window.open(consoleUrl, "_blank")}
					>
						<TerminalIcon className="h-4 w-4" />
						Terminal
					</Button>
				)}
				<Button
					variant="ghost"
					size="sm"
					className="gap-1.5 cursor-pointer"
					onClick={() => devServerRef.current?.refresh()}
				>
					<RefreshCwIcon className="h-4 w-4" />
					Refresh
				</Button>
				<ShareButton devServerUrl={vmUrl} />
			</div>
			<FreestyleDevServer
				ref={devServerRef}
				actions={{ requestDevServer }}
				repoId={repoId}
				loadingComponent={({ iframeLoading, devCommandRunning, installCommandRunning, serverStarting }) => {
					let status = "Starting VM";
					if (installCommandRunning) {
						status = "Installing dependencies";
					} else if (serverStarting) {
						status = "Starting dev server";
					} else if (devCommandRunning && iframeLoading) {
						status = "Loading preview";
					} else if (iframeLoading) {
						status = "Loading";
					}
					return (
						<div className="flex items-center justify-center h-full">
							<div>
								<div className="text-center text-muted-foreground">
									{status}
								</div>
								<div className="mt-4">
									<div className="loader"></div>
								</div>
							</div>
						</div>
					);
				}}
			/>
		</div>
	);
}
