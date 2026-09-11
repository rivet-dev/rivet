import {
	context,
	createTraceState,
	isSpanContextValid,
	propagation,
	type SpanContext,
	trace,
} from "@opentelemetry/api";
import type { ActorInvocationSpanContext } from "./actor-telemetry-context";

/** W3C headers derived from the active JavaScript OTel context. */
export interface ActiveTraceHeaders {
	/** W3C Trace Context identifying the active trace and span. */
	readonly traceparent: string;
	/** Optional vendor trace state associated with the active span. */
	readonly tracestate?: string;
}

/** Returns the active W3C trace context, when an OTel provider has installed one. */
export function readActiveTraceHeaders(): ActiveTraceHeaders | undefined {
	return traceHeaders(trace.getSpanContext(context.active()));
}

/** Serializes the structured span context supplied by Core as W3C headers. */
export function actorInvocationTraceHeaders(
	invocation: ActorInvocationSpanContext | undefined,
): ActiveTraceHeaders | undefined {
	const spanContext = invocationSpanContext(invocation);
	if (!spanContext) return undefined;
	return traceHeaders(spanContext);
}

/** W3C Baggage key that carries a ray ID through application code. */
export const RAY_BAGGAGE_KEY = "rivet.ray.id";

const RAY_ID_PATTERN = /^[A-Za-z0-9_-]{1,128}$/;

/**
 * Returns the ray ID carried in the active OpenTelemetry baggage, so a request
 * handler that received a ray ID can pass it to the actors it calls. The value is
 * bounded by the same rule Core applies at the actor edge: 1 to 128 characters
 * of `[A-Za-z0-9_-]`. Anything else counts as absent.
 */
export function readActiveRayId(): string | undefined {
	const rayId = propagation
		.getBaggage(context.active())
		?.getEntry(RAY_BAGGAGE_KEY)?.value;
	if (rayId === undefined || !RAY_ID_PATTERN.test(rayId)) return undefined;
	return rayId;
}

/**
 * Runs `run` with the Core invocation span as the active OpenTelemetry span,
 * so application spans started inside an actor callback nest under it. With
 * no span, or an invalid one, `run` executes unchanged.
 */
export function runWithActorInvocationSpan<T>(
	invocation: ActorInvocationSpanContext | undefined,
	run: () => T,
): T {
	if (!invocation) return run();

	const spanContext = invocationSpanContext(invocation);
	if (!spanContext) return run();
	const parent = trace.setSpanContext(context.active(), spanContext);

	return context.with(parent, run);
}

function invocationSpanContext(
	invocation: ActorInvocationSpanContext | undefined,
): SpanContext | undefined {
	if (!invocation) return undefined;
	try {
		const spanContext: SpanContext = {
			traceId: invocation.traceId,
			spanId: invocation.spanId,
			traceFlags: invocation.traceFlags,
			traceState: invocation.tracestate
				? createTraceState(invocation.tracestate)
				: undefined,
			isRemote: false,
		};
		return isSpanContextValid(spanContext) ? spanContext : undefined;
	} catch {
		return undefined;
	}
}

function traceHeaders(
	spanContext: SpanContext | undefined,
): ActiveTraceHeaders | undefined {
	if (!spanContext || !isSpanContextValid(spanContext)) return undefined;
	const tracestate = spanContext.traceState?.serialize();
	return {
		traceparent: `00-${spanContext.traceId}-${spanContext.spanId}-${(spanContext.traceFlags & 1).toString(16).padStart(2, "0")}`,
		...(tracestate ? { tracestate } : {}),
	};
}
