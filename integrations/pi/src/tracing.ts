import type { AgentSession } from "@earendil-works/pi-coding-agent";
import { context, SpanKind, SpanStatusCode, trace } from "@opentelemetry/api";

/**
 * Runs one Pi run inside an `invoke_agent pi` span, a child of the active span,
 * which is the `prompt` action span when RivetKit tracing is on. The action
 * span cannot show the outcome, because a model error does not reject the run,
 * so this span records it with OpenTelemetry GenAI attributes. Prompt text,
 * tool arguments, and tool results are not recorded.
 */
export async function traceRun(
	session: AgentSession,
	actorId: string,
	run: () => Promise<void>,
): Promise<void> {
	if (session.isStreaming) return run();
	const span = trace.getTracer("@rivet-dev/pi").startSpan("invoke_agent pi", {
		kind: SpanKind.INTERNAL,
		attributes: {
			"gen_ai.operation.name": "invoke_agent",
			"gen_ai.agent.name": "pi",
			"gen_ai.conversation.id": session.sessionId,
			"gen_ai.request.model": session.model?.id,
			"rivet.actor.id": actorId,
		},
	});
	try {
		await context.with(trace.setSpan(context.active(), span), run);
		const last = session.messages.findLast((message) => message.role === "assistant");
		if (last?.role === "assistant" && last.stopReason === "error") {
			span.setAttribute("error.type", "model_error");
			span.setStatus({ code: SpanStatusCode.ERROR, message: last.errorMessage });
		}
	} catch (error) {
		span.setAttribute("error.type", "run_error");
		span.setStatus({ code: SpanStatusCode.ERROR });
		throw error;
	} finally {
		span.end();
	}
}
