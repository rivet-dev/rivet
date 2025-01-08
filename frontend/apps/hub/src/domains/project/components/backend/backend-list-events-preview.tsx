import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
	Text,
	useBreakpoint,
} from "@rivet-gg/components";
import type { BackendEvent } from "../../queries";
import { BackendEventDetailsPanel } from "./backend-event-details-panel";
import { BackendEventsListPanel } from "./backend-events-list-panel";

interface BackendListEventsPreviewProps {
	events: BackendEvent[];
	environmentId: string;
	projectId: string;
	eventId?: string;
}

export function BackendListEventsPreview({
	events,
	eventId,
	projectId,
	environmentId,
}: BackendListEventsPreviewProps) {
	if (events.length === 0) {
		return (
			<Text my="10" textAlign="center">
				No events found.
			</Text>
		);
	}

	const isMd = useBreakpoint("md");

	return (
		<ResizablePanelGroup
			className="min-w-0 w-full h-full max-h-full"
			autoSaveId="rivet-project-backend-logs"
			direction={isMd ? "horizontal" : "vertical"}
		>
			<ResizablePanel minSize={25} maxSize={75}>
				<div className="h-full max-h-full overflow-hidden w-full truncate min-w-0">
					<BackendEventsListPanel events={events} eventId={eventId} />
				</div>
			</ResizablePanel>
			<ResizableHandle withHandle />
			<ResizablePanel minSize={25} maxSize={75}>
				<div className="h-full max-h-full overflow-hidden w-full">
					<BackendEventDetailsPanel
						environmentId={environmentId}
						projectId={projectId}
						eventId={eventId}
					/>
				</div>
			</ResizablePanel>
		</ResizablePanelGroup>
	);
}
