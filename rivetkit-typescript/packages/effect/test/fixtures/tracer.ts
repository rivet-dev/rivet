import { Context, Effect, Layer, Tracer } from "effect";

/**
 * Test-only tracer service: tests yield it to inspect spans recorded
 * during a call (`spans`) and reset between runs (`clear`).
 *
 * `TestTracer.layer()` overrides the active `Tracer.Tracer` Reference
 * with a wrapper around `Effect.tracer` that pushes every created span
 * into a buffer local to the layer closure. Because `Tracer.Tracer` is
 * a `Context.Reference` (always available via its default), the override
 * does not surface in the layer's output type; only the read-side
 * `TestTracer` service does.
 */
export class TestTracer extends Context.Service<
	TestTracer,
	{
		readonly spans: Effect.Effect<ReadonlyArray<Tracer.Span>>;
		readonly clear: Effect.Effect<void>;
	}
>()("test/TestTracer") {
	static layer() {
		return Layer.effectContext(
			Effect.gen(function* () {
				const buffer: Tracer.Span[] = [];
				const currentTracer = yield* Effect.tracer;
				const tracer = Tracer.make({
					span(options) {
						const span = currentTracer.span(options);
						buffer.push(span);
						return span;
					},
					context: currentTracer.context,
				});
				return Context.make(
					TestTracer,
					TestTracer.of({
						spans: Effect.sync(() => buffer.slice()),
						clear: Effect.sync(() => {
							buffer.length = 0;
						}),
					}),
				).pipe(Context.add(Tracer.Tracer, tracer));
			}),
		);
	}
}
