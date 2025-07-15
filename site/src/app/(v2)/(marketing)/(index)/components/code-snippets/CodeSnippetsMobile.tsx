"use client";

import {
	Icon,
	faGithub,
	faFileZip,
	faBolt,
	faCode,
} from "@rivet-gg/icons";
import {
	examples,
	type ExampleData,
} from "@/data/examples/examples";
import { EXAMPLE_ICON_MAP, createExampleActions } from "./utils";

interface ExampleListItemProps {
	example: ExampleData;
	icon: any;
}

function ExampleListItem({ example, icon }: ExampleListItemProps) {
	const { handleOpenGithub, handleOpenStackBlitz, handleDownloadZip } = 
		createExampleActions(example.id, example.files);

	return (
		<div className="border border-white/10 rounded-lg p-4 bg-white/[0.02]">
			<div className="flex items-center gap-3 mb-3">
				<Icon icon={icon} className="w-5 h-5 text-white/60" />
				<div>
					<h3 className="text-white font-medium text-sm">{example.title}</h3>
					<p className="text-white/60 text-xs">{example.description}</p>
				</div>
			</div>
			<div className="flex gap-2">
				<button
					onClick={handleOpenGithub}
					className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 border border-white/10 hover:border-white/20 rounded-md transition-all duration-200 flex-1 justify-center"
				>
					<Icon icon={faGithub} className="w-3 h-3" />
					GitHub
				</button>
				<button
					onClick={handleDownloadZip}
					className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 border border-white/10 hover:border-white/20 rounded-md transition-all duration-200 flex-1 justify-center"
				>
					<Icon icon={faFileZip} className="w-3 h-3" />
					ZIP
				</button>
				<button
					onClick={handleOpenStackBlitz}
					className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 border border-white/10 hover:border-white/20 rounded-md transition-all duration-200 flex-1 justify-center"
				>
					<Icon icon={faBolt} className="w-3 h-3" />
					StackBlitz
				</button>
			</div>
		</div>
	);
}

export default function CodeSnippetsMobile() {
	const examplesWithIcons = examples.map((example) => ({
		...example,
		icon: EXAMPLE_ICON_MAP[example.id] || faCode,
	}));

	return (
		<div className="p-4">
			<div className="space-y-4">
				{examplesWithIcons.map((example) => (
					<ExampleListItem
						key={example.id}
						example={example}
						icon={example.icon}
					/>
				))}
			</div>
		</div>
	);
}