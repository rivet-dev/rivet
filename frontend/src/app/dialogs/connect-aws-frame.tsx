import { faAws, Icon } from "@rivet-gg/icons";
import { type DialogContentProps, Frame } from "@/components";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";

interface ConnectAwsFrameContentProps extends DialogContentProps {
	footer?: React.ReactNode;
}

export default function ConnectAwsFrameContent({
	onClose,
	footer,
}: ConnectAwsFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faAws} className="ml-0.5" /> AWS ECS
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<ConnectManualServerlfullFrameContent
					provider="aws"
					onClose={onClose}
					footer={footer}
				/>
			</Frame.Content>
		</>
	);
}
