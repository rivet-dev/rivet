import { faServer, Icon } from "@rivet-gg/icons";
import { useState } from "react";
import {
	type DialogContentProps,
	Frame,
	Tabs,
	ToggleGroup,
	ToggleGroupItem,
} from "@/components";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";
import ConnectManualServerlessFrameContent from "./connect-manual-serverless-frame";

interface CreateProjectFrameContentProps extends DialogContentProps {}

export default function CreateProjectFrameContent({
	onClose,
}: CreateProjectFrameContentProps) {
	const [mode, setMode] = useState("serverfull");

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faServer} className="ml-0.5" /> Custom
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<div className="flex mx-auto items-center justify-center">
					<ToggleGroup
						defaultValue="serverfull"
						type="single"
						className="border rounded-md gap-0"
						value={mode}
						onValueChange={(mode) => {
							if(!mode) {
								return;
							}
							setMode(mode);
						}}
					>
						<ToggleGroupItem
							value="serverless"
							className="rounded-none"
						>
							Serverless
						</ToggleGroupItem>
						<ToggleGroupItem
							value="serverfull"
							className="border-l rounded-none"
						>
							Server
						</ToggleGroupItem>
					</ToggleGroup>
				</div>
				{mode === "serverless" ? (
					<ConnectManualServerlessFrameContent onClose={onClose} />
				) : null}
				{mode === "serverfull" ? (
					<ConnectManualServerlfullFrameContent
						onClose={onClose}
						provider="custom"
					/>
				) : null}
			</Frame.Content>
		</>
	);
}
