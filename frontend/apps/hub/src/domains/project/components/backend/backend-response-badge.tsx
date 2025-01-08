import { Badge } from "@rivet-gg/components";
import type { BackendEvent } from "../../queries";

const getResponseType = (event: BackendEvent["event"]) => {
	if (event.response.status >= 300 && event.response.status < 400) {
		return "warning";
	}
	if (event.response.status >= 400) {
		return "error";
	}
	return "success";
};

const getResponseTypeVariant = (type: "warning" | "error" | "success") => {
	if (type === "warning") {
		return "warning";
	}
	if (type === "error") {
		return "destructive-muted";
	}
	return "outline";
};

const getResponseLabel = (outcome: string, type: string) => {
	if (outcome.startsWith("exceeded")) {
		return "TIMEOUT";
	}
	if (outcome === "canceled") {
		return "CANCELED";
	}
	if (type === "error") {
		return "ERROR";
	}
	return "OK";
};

interface BackendResponseBadgeProps extends BackendEvent {}

export function BackendResponseBadge({
	backendCall,
	event,
	outcome,
}: BackendResponseBadgeProps) {
	const type = getResponseType(event);
	const variant = getResponseTypeVariant(type);

	const label = getResponseLabel(outcome, type);

	if (backendCall) {
		return (
			<>
				<Badge>CALL</Badge>
				<Badge variant={variant}>{label}</Badge>
			</>
		);
	}
	return (
		<>
			<Badge>HTTP</Badge>
			<Badge variant={variant}>
				{event.response.status} {label}
			</Badge>
		</>
	);
}
