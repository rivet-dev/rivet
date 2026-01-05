import { useState } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import "./App.css";
import DEFAULT_REGISTRY from "../../template/src/registry.ts?raw";
import DEFAULT_APP from "../../template/frontend/App.tsx?raw";
import { DeployRequest } from "../src/utils";

type DeploymentTarget = "cloud" | "selfHosted";

interface DeployConfig {
	target: DeploymentTarget;
	freestyleDomain: string;
	freestyleApiKey: string;
	// Cloud-specific
	cloudApiUrl: string;
	cloudApiToken: string;
	cloudEngineEndpoint: string;
	// Self-hosted specific
	selfHostedEndpoint: string;
	selfHostedToken: string;
}

export function App() {
	const [registryCode, setRegistryCode] = useState(DEFAULT_REGISTRY);
	const [appCode, setAppCode] = useState(DEFAULT_APP);
	const [deploying, setDeploying] = useState(false);
	const [deploymentLog, setDeploymentLog] = useState<string[]>([]);
	const [deploymentUrl, setDeploymentUrl] = useState<string | null>(null);
	const [dashboardUrl, setDashboardUrl] = useState<string | null>(null);
	const [freestyleUrl, setFreestyleUrl] = useState<string | null>(null);

	const handleEditorMount: OnMount = (_editor, monaco) => {
		// Disable TypeScript diagnostics since template code isn't valid standalone
		monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
			noSemanticValidation: true,
			noSyntaxValidation: true,
		});
	};

	const [config, setConfig] = useState<DeployConfig>({
		target: "cloud",
		freestyleDomain: "",
		freestyleApiKey: "",
		cloudApiUrl: import.meta.env.VITE_RIVET_CLOUD_ENDPOINT || "https://api-cloud.rivet.dev",
		cloudApiToken: "",
		cloudEngineEndpoint: import.meta.env.VITE_RIVET_ENGINE_ENDPOINT || "https://api.rivet.dev",
		selfHostedEndpoint: "",
		selfHostedToken: "",
	});

	const addLog = (message: string) => {
		setDeploymentLog((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${message}`]);
	};

	const handleDeploy = async () => {
		setDeploying(true);
		setDeploymentLog([]);
		setDeploymentUrl(null);
		setDashboardUrl(null);
		setFreestyleUrl(null);

		try {
			const datacenter = import.meta.env.VITE_RIVET_DATACENTER || "us-west-1";

			// Build request payload
			const payload: DeployRequest = {
				registryCode,
				appCode,
				datacenter,
				freestyleDomain: config.freestyleDomain,
				freestyleApiKey: config.freestyleApiKey,
				kind: config.target === "cloud"
					? {
						cloud: {
							cloudEndpoint: config.cloudApiUrl,
							cloudToken: config.cloudApiToken,
							engineEndpoint: config.cloudEngineEndpoint,
						}
					}
					: {
						selfHosted: {
							endpoint: config.selfHostedEndpoint,
							token: config.selfHostedToken,
						}
					}
			};

			const response = await fetch("/api/deploy", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(payload),
			});

			if (!response.ok) {
				throw new Error("Deployment request failed");
			}

			// Read SSE stream
			const reader = response.body?.getReader();
			if (!reader) {
				throw new Error("No response body");
			}

			const decoder = new TextDecoder();
			let buffer = "";

			while (true) {
				const { done, value } = await reader.read();
				if (done) break;

				buffer += decoder.decode(value, { stream: true });
				const lines = buffer.split("\n");
				buffer = lines.pop() || "";

				let currentEvent = "";
				for (const line of lines) {
					if (line.startsWith("event: ")) {
						currentEvent = line.slice(7);
					} else if (line.startsWith("data: ")) {
						const data = line.slice(6);
						if (currentEvent === "log") {
							addLog(data);
						} else if (currentEvent === "result") {
							const result = JSON.parse(data);
							setDeploymentUrl(`https://${config.freestyleDomain}/`);
							if (result.dashboardUrl) {
								setDashboardUrl(result.dashboardUrl);
							}
							if (result.freestyleUrl) {
								setFreestyleUrl(result.freestyleUrl);
							}
						} else if (currentEvent === "error") {
							addLog(`Error: ${data}`);
						}
						currentEvent = "";
					}
				}
			}
		} catch (error) {
			addLog(`Error: ${error instanceof Error ? error.message : String(error)}`);
		} finally {
			setDeploying(false);
		}
	};

	return (
		<div className="app">
			<div className="ide-layout">
				<div className="editors-row">
					<div className="editor-section">
						<div className="editor-header">src/backend/registry.ts</div>
						<Editor
							height="100%"
							defaultLanguage="typescript"
							value={registryCode}
							onChange={(value) => setRegistryCode(value || "")}
							onMount={handleEditorMount}
							theme="vs-dark"
							options={{
								minimap: { enabled: false },
								fontSize: 14,
								scrollBeyondLastLine: false,
								automaticLayout: true,
							}}
						/>
					</div>

					<div className="editor-section">
						<div className="editor-header">src/frontend/App.tsx</div>
						<Editor
							height="100%"
							defaultLanguage="typescript"
							value={appCode}
							onChange={(value) => setAppCode(value || "")}
							onMount={handleEditorMount}
							theme="vs-dark"
							options={{
								minimap: { enabled: false },
								fontSize: 14,
								scrollBeyondLastLine: false,
								automaticLayout: true,
							}}
						/>
					</div>
				</div>

				<div className="deploy-panel">
					<div className="panel-header">
						<h2>Deploy to Freestyle</h2>
					</div>

					<div className="tabs">
						<button
							className={config.target === "cloud" ? "active" : ""}
							onClick={() => setConfig({ ...config, target: "cloud" })}
						>
							Rivet Cloud
						</button>
						<button
							className={config.target === "selfHosted" ? "active" : ""}
							onClick={() => setConfig({ ...config, target: "selfHosted" })}
						>
							Rivet Self-Hosted
						</button>
					</div>

					<div className="panel-content">
						<div className="env-vars-section">
							<h3>Configuration</h3>
							<div className="env-vars">
								{config.target === "cloud" && (
									<div className="env-var">
										<label>Rivet Cloud API Token</label>
										<input
											type="password"
											value={config.cloudApiToken}
											onChange={(e) =>
												setConfig({ ...config, cloudApiToken: e.target.value })
											}
											placeholder="Required"
										/>
									</div>
								)}
								{config.target === "selfHosted" && (
									<>
										<div className="env-var">
											<label>Rivet Endpoint</label>
											<input
												type="text"
												value={config.selfHostedEndpoint}
												onChange={(e) =>
													setConfig({ ...config, selfHostedEndpoint: e.target.value })
												}
												placeholder="Required"
											/>
										</div>
										<div className="env-var">
											<label>Rivet Token</label>
											<input
												type="password"
												value={config.selfHostedToken}
												onChange={(e) =>
													setConfig({ ...config, selfHostedToken: e.target.value })
												}
												placeholder="Required"
											/>
										</div>
									</>
								)}
								<div className="env-var">
									<label>Freestyle Domain</label>
									<input
										type="text"
										value={config.freestyleDomain}
										onChange={(e) =>
											setConfig({ ...config, freestyleDomain: e.target.value })
										}
										placeholder="myapp.style.dev"
									/>
								</div>
								<div className="env-var">
									<label>Freestyle API Key</label>
									<input
										type="password"
										value={config.freestyleApiKey}
										onChange={(e) =>
											setConfig({ ...config, freestyleApiKey: e.target.value })
										}
										placeholder="Required"
									/>
								</div>
							</div>
						</div>

						<div className="deploy-section">
							<button
								className="deploy-button"
								onClick={handleDeploy}
								disabled={deploying}
							>
								{deploying ? "Deploying..." : "Deploy →"}
							</button>
						</div>

						{deploymentLog.length > 0 && (
							<div className={`log-section${deploying ? " deploying" : ""}`}>
								<h3>Deployment Log</h3>
								{deploymentLog.map((log, i) => (
									<div key={i}>{log}</div>
								))}
							</div>
						)}

						{deploymentUrl && (
							<div className="deployment-success">
								<h3>Deployment Complete</h3>
								<a href={deploymentUrl} target="_blank" rel="noopener noreferrer">
									Open App ›
								</a>
								{dashboardUrl && (
									<a href={dashboardUrl} target="_blank" rel="noopener noreferrer">
										Rivet Namespace ›
									</a>
								)}
								{freestyleUrl && (
									<a href={freestyleUrl} target="_blank" rel="noopener noreferrer">
										Freestyle Deployment ›
									</a>
								)}
							</div>
						)}
					</div>
				</div>
			</div>
		</div>
	);
}
