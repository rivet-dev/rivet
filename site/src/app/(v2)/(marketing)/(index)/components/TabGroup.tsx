"use client";

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
	Icon,
} from "@rivet-gg/icons";
import { type ExampleData, type StateTypeTab } from "@/data/examples/examples";

const EXAMPLE_ICON_MAP: Record<string, any> = {
	"ai-agent": faRobot,
	"chat-room": faMessage,
	crdt: faFilePen,
	database: faDatabase,
	rate: faGaugeHigh,
	stream: faWaveSine,
	game: faGamepad,
	sync: faRotate,
	tenant: faBuilding,
};

interface TabGroupProps {
	examples: ExampleData[];
	activeExample: string;
	setActiveExample: (example: string) => void;
	activeStateType: StateTypeTab;
	setActiveStateType: (state: StateTypeTab) => void;
}

export default function TabGroup({
	examples,
	activeExample,
	setActiveExample,
	activeStateType,
	setActiveStateType,
}: TabGroupProps) {
	// Transform examples data to include actual icon components
	const examplesWithIcons = examples.map((example) => ({
		...example,
		icon: EXAMPLE_ICON_MAP[example.id] || faCode,
	}));

	return (
		<div className="border-b border-white/10">
			{/* Example Tabs */}
			<div className="px-6 py-4 border-b border-white/5">
				<div className="flex items-center gap-1 text-sm text-white/40 mb-3">
					<span className="font-medium">Example</span>
				</div>
				<div className="flex gap-2 overflow-x-auto scrollbar-hide flex-1">
					{examplesWithIcons.map((example) => (
						<button
							key={example.id}
							onClick={() => setActiveExample(example.id)}
							className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-200 ${
								activeExample === example.id
									? "bg-white/10 text-white border border-white/20"
									: "text-white/60 hover:text-white/80 hover:bg-white/5"
							}`}
						>
							<Icon icon={example.icon as any} className="w-3.5 h-3.5" />
							{example.title}
						</button>
					))}
				</div>
			</div>

		</div>
	);
}