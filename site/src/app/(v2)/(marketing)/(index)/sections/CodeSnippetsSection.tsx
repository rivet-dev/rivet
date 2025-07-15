"use client";

import { useState, useEffect, useRef } from "react";
import {
	faFilePen,
	faRobot,
	faMessage,
	faDatabase,
	faGaugeHigh,
	faWaveSine,
	faGamepad,
	faRotate,
	faBuilding,
	faCode,
	Icon
} from "@rivet-gg/icons";
import sdk, { VM, Project } from "@stackblitz/sdk";
import examplesData from "@/data/examples/examples.json";

type StateTab = "memory" | "sqlite";

// Get available examples dynamically from the JSON data
const availableExamples = Object.keys(examplesData);
type ExampleTab = typeof availableExamples[number];

export function CodeSnippetsSection() {
	const [activeExample, setActiveExample] = useState<ExampleTab>(availableExamples[0] as ExampleTab);
	const [activeState, setActiveState] = useState<StateTab>("memory");
	const [vm, setVm] = useState<VM | null>(null);
	const embedRef = useRef<HTMLDivElement>(null);

	// Generate example metadata dynamically
	const getExampleMetadata = (exampleId: string) => {
		const metadata: Record<string, { icon: any; title: string }> = {
			"ai": { icon: faRobot, title: "AI Agent" },
			"ai-agent": { icon: faRobot, title: "AI Agent" },
			"crdt": { icon: faFilePen, title: "Collaborative Document (CRDT)" },
			"chat": { icon: faMessage, title: "Chat Room" },
			"chat-room": { icon: faMessage, title: "Chat Room" },
			"database": { icon: faDatabase, title: "Per-User Databases" },
			"rate": { icon: faGaugeHigh, title: "Rate Limiter" },
			"stream": { icon: faWaveSine, title: "Stream Processing" },
			"game": { icon: faGamepad, title: "Multiplayer Game" },
			"sync": { icon: faRotate, title: "Local-First Sync" },
			"tenant": { icon: faBuilding, title: "Multi-Tenant Architecture" },
		};

		return metadata[exampleId] || {
			icon: faCode,
			title: exampleId.charAt(0).toUpperCase() + exampleId.slice(1).replace(/-/g, ' ')
		};
	};

	const examples = availableExamples.map(exampleId => ({
		id: exampleId as ExampleTab,
		...getExampleMetadata(exampleId)
	}));

	// Create StackBlitz project from static examples
	const createProject = (example: ExampleTab, state: StateTab): Project => {
		const exampleData = examplesData[example];
		if (!exampleData) {
			throw new Error(`Example ${example} not found`);
		}

		// Use all files from the example data as-is
		const files: Record<string, string> = { ...exampleData };

		return {
			title: `Rivet ${example.charAt(0).toUpperCase() + example.slice(1)} Example`,
			description: `A ${example} example using Rivet Actors`,
			template: "node",
			files,
			settings: {
				compile: {
					clearConsole: false,
				},
			},
		};
	};

	// Get files to open for the current example and state
	const getFilesToOpen = (example: ExampleTab, state: StateTab): string[] => {
		const exampleData = examplesData[example];
		if (!exampleData) return [];

		// Get the main files that should be opened by default
		const filesToOpen = [];
		const fileNames = Object.keys(exampleData);

		// Look for main entry files
		const mainFiles = ['index.ts', 'index.tsx', 'main.ts', 'main.tsx', 'App.tsx', 'actor.ts'];

		for (const mainFile of mainFiles) {
			if (fileNames.includes(mainFile)) {
				filesToOpen.push(mainFile);
			}
		}

		// If no main files found, just open the first few files
		if (filesToOpen.length === 0) {
			filesToOpen.push(...fileNames.slice(0, 2));
		}

		return filesToOpen;
	};

	// Initialize StackBlitz embed
	useEffect(() => {
		if (!embedRef.current) return;

		const embedProject = async () => {
			try {
				const project = createProject(activeExample, activeState);
				const stackblitzVm = await sdk.embedProject(embedRef.current!, project, {
					openFile: getFilesToOpen(activeExample, activeState),
					view: "editor",
					theme: "dark",
					height: 600,
					hideNavigation: true,
					hideDevTools: true,
					hideExplorer: false,
					forceEmbedLayout: true,
				});

				setVm(stackblitzVm);
			} catch (error) {
				console.error("Failed to embed StackBlitz project:", error);
			}
		};

		embedProject();
	}, [activeExample, activeState]);

	return (
		<div className="mx-auto max-w-7xl">
			<div className="text-center mb-16">
				<h2 className="text-4xl sm:text-5xl font-700 text-white mb-6">
					Reconsider What Your Backend Can Do
				</h2>
				<p className="text-lg sm:text-xl font-500 text-white/60 max-w-3xl mx-auto">
					Build powerful applications with Rivet Actors
				</p>
			</div>

			<div className="bg-white/5 backdrop-blur border border-white/10 rounded-2xl overflow-hidden">
				{/* Tabs */}
				<div className="border-b border-white/10">
					{/* Example Tabs */}
					<div className="px-6 py-4 border-b border-white/5">
						<div className="flex items-center gap-1 text-sm text-white/40 mb-3">
							<span className="font-medium">Example</span>
						</div>
						<div className="flex gap-2 overflow-x-auto scrollbar-hide">
							{examples.map((example) => (
								<button
									key={example.id}
									onClick={() => setActiveExample(example.id)}
									className={`px-3 py-1.5 rounded-lg text-sm font-medium flex items-center gap-2 transition-all duration-200 ${
										activeExample === example.id
											? "bg-white/10 text-white"
											: "text-white/60 hover:text-white/80 hover:bg-white/5"
									}`}
								>
							<Icon icon={example.icon as any} className="w-3.5 h-3.5" />
							{example.title}
						</button>
							))}
					</div>
				</div>

				{/* State Tabs */}
				<div className="px-6 py-3">
					<div className="flex items-center gap-4">
						<span className="text-sm text-white/40 font-medium">State</span>
						<div className="flex gap-2">
							<button
								onClick={() => setActiveState("memory")}
								className={`px-3 py-1 rounded text-sm font-medium transition-all duration-200 ${activeState === "memory"
									? "bg-white/10 text-white"
									: "text-white/60 hover:text-white/80"
									}`}
							>
								JavaScript
							</button>
							<button
								onClick={() => setActiveState("sqlite")}
								className={`px-3 py-1 rounded text-sm font-medium transition-all duration-200 relative ${activeState === "sqlite"
									? "bg-white/10 text-white"
									: "text-white/60 hover:text-white/80"
									}`}
							>
								SQLite
								<span className="ml-2 text-xs text-orange-400 font-normal">
									Available In July
								</span>
							</button>
						</div>
					</div>
				</div>
			</div>

			{/* StackBlitz Code Editor */}
			<div className="h-[600px] w-full">
				<div
					ref={embedRef}
					className="w-full h-full rounded-b-2xl overflow-hidden"
					style={{ minHeight: "600px" }}
				/>
			</div>
		</div>
		</div >
	);
}
