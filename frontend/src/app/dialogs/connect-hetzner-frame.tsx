import { faHetznerH, Icon } from "@rivet-gg/icons";
import { type DialogContentProps, Frame } from "@/components";
import ConnectManualServerlfullFrameContent from "./connect-manual-serverfull-frame";

interface ConnectHetznerFrameContentProps extends DialogContentProps {
	footer?: React.ReactNode;
}

export default function ConnectHetznerFrameContent({
	onClose,
	footer,
}: ConnectHetznerFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faHetznerH} className="ml-0.5" />{" "}
						Hetzner
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<ConnectManualServerlfullFrameContent
					provider="hetzner"
					footer={footer}
					onClose={onClose}
				/>
			</Frame.Content>
		</>
	);
}
