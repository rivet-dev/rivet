import { faGoogleCloud, Icon } from "@rivet-gg/icons";
import { type DialogContentProps, Frame } from "@/components";
import ConnectManualServerlessFrameContent from "./connect-manual-serverless-frame";

interface ConnectGcpFrameContentProps extends DialogContentProps {
	footer?: React.ReactNode;
	title?: React.ReactNode;
}

export default function ConnectGcpFrameContent({
	onClose,
	title,
}: ConnectGcpFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					{title ?? (
						<div>
							Add <Icon icon={faGoogleCloud} className="ml-0.5" />{" "}
							Google Cloud Run
						</div>
					)}
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<ConnectManualServerlessFrameContent
					provider="gcp-cloud-run"
					onClose={onClose}
				/>
			</Frame.Content>
		</>
	);
}
