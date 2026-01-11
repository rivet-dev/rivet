import type { Rivet } from "@rivetkit/engine-api-full";
import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { match, P } from "ts-pattern";
import { RelativeTime } from "../relative-time";
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
	const { data: status, isError } = useQuery(
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
	const { data: { rescheduleAt, error } = {} } = useQuery(
		useDataProvider().actorStatusAdditionalInfoQueryOptions(actorId),
	);

	if (rescheduleAt) {
		return (
			<span>
				Will try to start again{" "}
				<span>
					<RelativeTime time={new Date(rescheduleAt)} /> (
					{formatISO(rescheduleAt)}){" "}
				</span>
			</span>
		);
	}

	if (error) {
		return <ActorError error={error} />;
	}

	return null;
}

export function ActorError({ error }: { error: Rivet.ActorError }) {
	return match(error)
		.with(P.string, (errMsg) =>
			match(errMsg)
				.with("no_capacity", () => (
					<p>No capacity available to start Actor.</p>
				))
				.otherwise(() => <p>Unknown error: {errMsg}</p>),
		)
		.with(P.shape({ runnerPoolError: P.any }), (err) => (
			<p>
				Runner Pool Error:{" "}
				<RunnerPoolError error={err.runnerPoolError} />
			</p>
		))
		.with(P.shape({ runnerNoResponse: P.any }), (err) => (
			<p>
				Runner ({err.runnerNoResponse.runnerId}) was allocated but Actor
				did not respond.
			</p>
		))
		.with(P.shape({ runnerConnectionLost: P.any }), (err) => (
			<p>
				Runner ({err.runnerConnectionLost.runnerId}) connection was lost
				(no recent ping, network issue, or crash).
			</p>
		))
		.with(P.shape({ runnerDrainingTimeout: P.any }), (err) => (
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
	error: Rivet.RunnerPoolError | undefined;
}) {
	return match(error)
		.with(P.nullish, () => null)
		.with(P.string, (errStr) =>
			match(errStr)
				.with(
					"internal_error",
					() => "Internal error occurred in runner pool",
				)
				.with(
					"serverless_invalid_base64",
					() => "Invalid base64 encoding in serverless response",
				)
				.with(
					"serverless_stream_ended_early",
					() => "Connection terminated unexpectedly",
				)
				.otherwise(() => "Unknown runner pool error"),
		)
		.with(P.shape({ serverlessHttpError: P.any }), (errObj) => {
			const { statusCode, body } = errObj.serverlessHttpError;
			const code = statusCode ?? "unknown";
			return body ? `HTTP ${code} error: ${body}` : `HTTP ${code} error`;
		})
		.with(P.shape({ serverlessConnectionError: P.any }), (errObj) => {
			const message = errObj.serverlessConnectionError?.message;
			return message
				? `Connection failed: ${message}`
				: "Unable to connect to serverless endpoint";
		})
		.with(P.shape({ serverlessInvalidPayload: P.any }), (errObj) => {
			const message = errObj.serverlessInvalidPayload?.message;
			return message
				? `Invalid request payload: ${message}`
				: "Request payload validation failed";
		})
		.exhaustive();
}
