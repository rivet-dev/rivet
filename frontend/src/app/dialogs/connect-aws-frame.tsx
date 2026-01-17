import { faAws, Icon } from "@rivet-gg/icons";
import { type DialogContentProps, Frame } from "@/components";
import ConnectManualServerlessFrameContent from "./connect-manual-serverless-frame";

interface ConnectAwsFrameContentProps extends DialogContentProps {
	footer?: React.ReactNode;
	title?: React.ReactNode;
}

export default function ConnectAwsFrameContent({
	onClose,
	title,
}: ConnectAwsFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					{title ?? (
						<div>
							Add <Icon icon={faAws} className="ml-0.5" /> AWS ECS
						</div>
					)}
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<ConnectManualServerlessFrameContent
					provider="aws-ecs"
					onClose={onClose}
				/>
			</Frame.Content>
		</>
	);
}
