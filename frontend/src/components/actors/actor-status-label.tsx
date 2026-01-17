import type { Rivet } from "@rivetkit/engine-api-full";
import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { isObject } from "lodash";
import { match, P } from "ts-pattern";
import { CodePreview } from "../code-preview/code-preview";
import { RelativeTime } from "../relative-time";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "../ui/accordion";
import { ScrollArea } from "../ui/scroll-area";
import { Code } from "../ui/typography";
import { useDataProvider } from "./data-provider";
import type { ActorId, ActorStatus } from "./queries";

export const ACTOR_STATUS_LABEL_MAP = {
	unknown: "Unknown",
	starting: "Starting",
	running: "Running",
	stopped: "Destroyed",
	crashed: "Crashed",
	sleeping: "Sleeping",
	pending: "Pending",
	"crash-loop": "Crash Loop Backoff",
} satisfies Record<ActorStatus, string>;

export const ActorStatusLabel = ({ status }: { status?: ActorStatus }) => {
	return (
		<span className="whitespace-nowrap">
			{status ? ACTOR_STATUS_LABEL_MAP[status] : "Unknown"}
		</span>
	);
};

export const QueriedActorStatusLabel = ({
	actorId,
	showAdditionalInfo,
}: {
	actorId: ActorId;
	showAdditionalInfo?: boolean;
}) => {
	const { data: status = "unknown", isError } = useQuery(
		useDataProvider().actorStatusQueryOptions(actorId),
	);
	return (
		<>
			<ActorStatusLabel status={isError ? "unknown" : status} />
			{showAdditionalInfo && (
				<QueriedActorStatusAdditionalInfo actorId={actorId} />
			)}
		</>
	);
};

export function QueriedActorStatusAdditionalInfo({
	actorId,
}: {
	actorId: ActorId;
}) {
	const { data: { rescheduleTs, error } = {} } = useQuery(
		useDataProvider().actorStatusAdditionalInfoQueryOptions(actorId),
	);

	if (rescheduleTs) {
		return (
			<span>
				Will try to start again{" "}
				<span>
					<RelativeTime time={new Date(rescheduleTs)} /> (
					{formatISO(rescheduleTs)}){" "}
				</span>
			</span>
		);
	}

	if (error) {
		return <ActorError error={error} />;
	}

	return null;
}

export function ActorError({ error }: { error: object | string }) {
	return match(error)
		.with(P.string, (errMsg) =>
			match(errMsg)
				.with("no_capacity", () => (
					<p>No capacity available to start Actor.</p>
				))
				.with("internal_error", () => (
					<p>Actor has an internal error.</p>
				))
				.otherwise(() => <p>Unknown error: {errMsg}</p>),
		)
		.with(P.shape({ runnerPoolError: P.shape({ runnerId: P.string }) }), (err) => (
			<RunnerPoolError error={err.runnerPoolError} />
		))
		.with(P.shape({ runnerNoResponse: P.shape({ runnerId: P.string }) }), (err) => (
			<p>
				Runner ({err.runnerNoResponse.runnerId}) was allocated but Actor
				did not respond.
			</p>
		))
		.with(P.shape({ runnerConnectionLost: P.shape({ runnerId: P.string }) }), (err) => (
			<p>
				Runner ({err.runnerConnectionLost.runnerId}) connection was lost
				(no recent ping, network issue, or crash).
			</p>
		))
		.with(P.shape({ runnerDrainingTimeout: P.shape({ runnerId: P.string }) }), (err) => (
			<p>
				Runner ({err.runnerDrainingTimeout.runnerId}) was draining but
				Actor didn't stop in time.
			</p>
		))
		.with(P.shape({ crashed: P.any }), () => (
			<p>Actor exited with an error and is now sleeping.</p>
		))
		.otherwise(() => {
			return <p>Unknown error.</p>;
		});
}

export function QueriedActorError({ actorId }: { actorId: ActorId }) {
	const { data: error, isError } = useQuery(
		useDataProvider().actorErrorQueryOptions(actorId),
	);

	if (isError || !error) {
		return null;
	}

	return <ActorError error={error} />;
}

export function RunnerPoolError({
	error,
}: {
	error: object | string | undefined;
}) {
	return match(error)
		.with(P.nullish, () => null)
		.with(P.string, (errStr) =>
			match(errStr)
				.with("internal_error", () => (
					<p>Internal error occurred in runner pool</p>
				))
				.with("serverless_stream_ended_early", () => (
					<p>Connection terminated unexpectedly</p>
				))
				.otherwise(() => <p>Unknown runner pool error</p>),
		)
		.with(P.shape({ serverlessHttpError: P.shape({ statusCode: P.number, body: P.string }) }), (errObj) => {
			const { statusCode, body } = errObj.serverlessHttpError;
			const code = statusCode ?? "unknown";
			return (
				<>
					<p>Serverless HTTP error with status code {code}</p>
					{body ? <ErrorDetails error={body} /> : null}
				</>
			);
		})
		.with(P.shape({ serverlessConnectionError: P.shape({ message: P.string }) }), (errObj) => {
			const message = errObj.serverlessConnectionError?.message;
			return (
				<>
					<p>Unable to connect to serverless endpoint</p>
					{message ? <ErrorDetails error={message} /> : null}
				</>
			);
		})
		.with(P.shape({ serverlessInvalidSsePayload: P.shape({ message: P.string }) }), (errObj) => {
			const message = errObj.serverlessInvalidSsePayload?.message;
			return (
				<>
					<p>Request payload validation failed</p>
					{message ? <ErrorDetails error={message} /> : null}
				</>
			);
		})
		.otherwise(() => {
			return <p>Unknown runner pool error.</p>;
		});
}

export function ErrorDetails({ error }: { error: unknown }) {
	const json =
		typeof error === "string"
			? tryJsonParse(error)
			: isObject(error)
				? error
				: null;
	return (
		<Accordion
			type="single"
			collapsible
			className="mt-4 max-w-full min-w-0"
		>
			<AccordionItem value="error-details">
				<AccordionTrigger className="gap-1 p-0 max-w-full min-w-0">
					View Error Details
				</AccordionTrigger>
				<AccordionContent className="max-w-full min-w-0 ">
					{json ? (
						<div className="not-prose my-4 rounded-lg border p-1 bg-background">
							<ScrollArea className="w-full">
								<CodePreview
									language="json"
									className="text-left"
									code={
										json
											? JSON.stringify(json, null, 2)
											: String(error)
									}
								/>
							</ScrollArea>
						</div>
					) : (
						<Code className="block whitespace-pre-wrap text-left">
							{String(error)}
						</Code>
					)}
				</AccordionContent>
			</AccordionItem>
		</Accordion>
	);
}

const tryJsonParse = (str: string) => {
	try {
		return JSON.parse(str);
	} catch {
		return null;
	}
};
