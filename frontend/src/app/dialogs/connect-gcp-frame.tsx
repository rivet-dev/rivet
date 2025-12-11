import { faGoogleCloud, Icon } from "@rivet-gg/icons";
import { type DialogContentProps, Frame } from "@/components";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";

interface ConnectGcpFrameContentProps extends DialogContentProps {
	footer?: React.ReactNode;
}

export default function ConnectGcpFrameContent({
	onClose,
	footer,
}: ConnectGcpFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faGoogleCloud} className="ml-0.5" />{" "}
						Google Cloud Run
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<ConnectManualServerlfullFrameContent
					provider="gcp"
					footer={footer}
					onClose={onClose}
				/>
			</Frame.Content>
		</>
	);
}
