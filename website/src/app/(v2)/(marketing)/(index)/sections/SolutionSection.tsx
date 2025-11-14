"use client";

import { useEffect } from "react";

export function SolutionSection() {
	useEffect(() => {
		// Sticky code highlighting with scroll-based step animations
		const codeSteps = document.querySelectorAll<HTMLDivElement>(".code-highlight-step");
		if (codeSteps.length === 0) return;

		const codeRefs = new Map(
			Array.from(document.querySelectorAll<HTMLElement>(".code-highlight-ref")).map((el) => [el.id, el])
		);
		let activeCodeRef: HTMLElement | null = null;
		let activeStep: HTMLElement | null = null;

		const codeObserver = new IntersectionObserver(
			(entries) => {
				entries.forEach((entry) => {
					const targetId = (entry.target as HTMLElement).dataset.targetCode;
					const codeEl = codeRefs.get(targetId || "");
					const stepEl = entry.target as HTMLElement;
					const stepIndex = Array.from(codeSteps).indexOf(stepEl as HTMLDivElement);
					const isFirstStep = stepIndex === 0;
					const isLastStep = stepIndex === codeSteps.length - 1;

					if (entry.isIntersecting) {
						// Hide all other steps first
						codeSteps.forEach((step) => {
							if (step !== stepEl) {
								const rect = step.getBoundingClientRect();
								const isAbove = rect.top < window.innerHeight / 2;
								step.style.opacity = "0";
								step.style.transform = isAbove
									? "translateY(-30px) scale(0.95)"
									: "translateY(30px) scale(0.95)";
							}
						});

						// Activate code highlighting - only deactivate previous when activating new one
						if (activeCodeRef && activeCodeRef !== codeEl) {
							activeCodeRef.classList.remove("is-active");
						}
						if (codeEl) {
							codeEl.classList.add("is-active");
							activeCodeRef = codeEl;
						}

						// Show step with animation (centered and visible)
						stepEl.style.opacity = "1";
						stepEl.style.transform = "translateY(0) scale(1)";
						activeStep = stepEl;
					} else {
						// Special handling for last step
						if (isLastStep) {
							// Last step: check if we've scrolled past it
							const rect = stepEl.getBoundingClientRect();
							const hasScrolledPast = rect.top < window.innerHeight / 2;
							if (hasScrolledPast) {
								// Keep last step visible and keep code highlighted
								stepEl.style.opacity = "1";
								stepEl.style.transform = "translateY(0) scale(1)";
								if (codeEl && !codeEl.classList.contains("is-active")) {
									if (activeCodeRef) activeCodeRef.classList.remove("is-active");
									codeEl.classList.add("is-active");
									activeCodeRef = codeEl;
								}
								activeStep = stepEl;
								return; // Don't hide it
							}
						}

						// Hide step with animation
						const rect = stepEl.getBoundingClientRect();
						const isAbove = rect.top < window.innerHeight / 2;
						stepEl.style.opacity = "0";
						stepEl.style.transform = isAbove
							? "translateY(-30px) scale(0.95)"
							: "translateY(30px) scale(0.95)";
					}
				});
			},
			{
				rootMargin: "-42% 0px -42% 0px",
				threshold: 0,
			}
		);

		// Initialize steps: first step visible and centered, others hidden
		codeSteps.forEach((step, index) => {
			step.style.transition = "opacity 0.35s ease-in-out, transform 0.35s ease-in-out";
			if (index === 0) {
				// First step starts visible and centered
				step.style.opacity = "1";
				step.style.transform = "translateY(0) scale(1)";
				activeStep = step;
				// Also activate the first code highlight
				const targetId = step.dataset.targetCode;
				const codeEl = codeRefs.get(targetId || "");
				if (codeEl) {
					codeEl.classList.add("is-active");
					activeCodeRef = codeEl;
				}
			} else {
				// Other steps start hidden and slightly offset
				step.style.opacity = "0";
				step.style.transform = "translateY(30px) scale(0.95)";
			}
			codeObserver.observe(step);
		});

		return () => {
			codeSteps.forEach((step) => codeObserver.unobserve(step));
		};
	}, []);

	return (
		<section className="relative pt-48 md:pt-64">
			{/* Centered Container */}
			<div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
				{/* Header */}
				<h2 className="text-center font-heading text-4xl md:text-5xl font-bold tracking-tighter text-text-primary mb-16">
					Stop Writing Glue Code.
				</h2>

				{/* Content Grid */}
				<div className="grid grid-cols-1 md:grid-cols-2 gap-8 md:gap-16">
					{/* Left Side: Sticky Code */}
					<div>
						<div className="sticky top-32 md:top-40 z-10">
							<div className="flex-none rounded-xl border border-border bg-black/50">
								<div className="flex items-center justify-between border-b border-border px-4 py-3">
									<span className="text-sm font-medium text-text-primary">ai-agent.ts</span>
									<span className="text-sm text-text-secondary font-mono">actor.ts</span>
								</div>
								<div className="p-6 font-mono text-sm overflow-x-auto">
								<pre>
									<code>
										<span className="code-comment">// Define your actor</span>
										{"\n"}
										<span className="code-keyword">export const</span> aiAgent ={" "}
										<span className="code-function">actor</span>({"{"}
										{"\n"}
										<span className="code-highlight-ref" id="code-ref-1">
											{"  "}
											<span className="code-comment">// Persistent state that survives restarts</span>
											{"\n"}
											{"  "}
											<span className="code-function">state</span>: {"{"}
											{"\n"}
											{"    "}messages: [] <span className="code-keyword">as</span> Message[],
											{"\n"}
											{"  "}{"},"}
										</span>
										{"\n"}
										<span className="code-highlight-ref" id="code-ref-2">
											{"  "}actions: {"{"}
											{"\n"}
											{"    "}
											<span className="code-comment">// Call functions from clients</span>
											{"\n"}
											{"    "}sendMessage: <span className="code-keyword">async</span> (c, userMessage:{" "}
											<span className="code-string">string</span>) ={">"} {"{"}
										</span>
										{"\n"}
										{"      "}
										<span className="code-keyword">const</span> userMsg = {"{"} role:{" "}
										<span className="code-string">"user"</span>, content: userMessage {"}"};
										{"\n"}
										{"      "}
										{"\n"}
										<span className="code-highlight-ref" id="code-ref-3">
											{"      "}
											<span className="code-comment">// State changes are automatically persisted</span>
											{"\n"}
											{"      "}
											<span className="text-accent font-bold">c.state</span>.messages.push(userMsg);
											{"\n"}
											{"      "}
											<span className="code-keyword">const</span> assistantMsg ={" "}
											<span className="code-keyword">await</span>{" "}
											<span className="code-function">getAIReply</span>(
											<span className="text-accent font-bold">c.state</span>.messages);
											{"\n"}
											{"      "}
											<span className="text-accent font-bold">c.state</span>.messages.push(assistantMsg);
										</span>
										{"\n"}
										<span className="code-highlight-ref" id="code-ref-4">
											{"      "}
											<span className="code-comment">// Send events to all connected clients</span>
											{"\n"}
											{"      "}c.<span className="code-function">broadcast</span>(
											<span className="code-string">"messageReceived"</span>, assistantMsg);
											{"\n"}
											{"      "}
											<span className="code-keyword">return</span> assistantMsg;
										</span>
										{"\n"}
										{"    "}{"},"}
										{"\n"}
										{"  "}{"},"}
										{"\n"}
										{"}"});
									</code>
								</pre>
							</div>
						</div>
					</div>
					</div>

					{/* Right Side: The Explanation (Scroll-triggers) */}
					<div className="mt-8 md:mt-0">
						<div className="flex flex-col items-center">
							<div className="h-[10vh]"></div>
							<div className="code-highlight-step text-center" data-target-code="code-ref-1">
								<h3 className="font-heading text-xl font-bold text-text-primary">1. Define Your State</h3>
								<p className="mt-2 text-text-secondary max-w-md mx-auto">
									Declare your state in a simple object. Rivet handles persistence, replication, and storage for you.
									It's just there.
								</p>
							</div>

							<div className="h-[20vh]"></div>
							<div className="code-highlight-step text-center" data-target-code="code-ref-2">
								<h3 className="font-heading text-xl font-bold text-text-primary">2. Add Your Actions</h3>
								<p className="mt-2 text-text-secondary max-w-md mx-auto">
									Write functions that clients can call. These are your entrypoints, just like a serverless function or
									API route.
								</p>
							</div>

							<div className="h-[20vh]"></div>
							<div className="code-highlight-step text-center" data-target-code="code-ref-3">
								<h3 className="font-heading text-xl font-bold text-text-primary">3. Mutate State In-Memory</h3>
								<p className="mt-2 text-text-secondary max-w-md mx-auto">
									Just modify{" "}
									<code className="font-mono text-sm text-accent bg-border rounded px-1 py-0.5">c.state</code>. Reads
									and writes are instant. No database round-trips, no `await`.
								</p>
							</div>

							<div className="h-[20vh]"></div>
							<div className="code-highlight-step text-center" data-target-code="code-ref-4">
								<h3 className="font-heading text-xl font-bold text-text-primary">4. Go Realtime, Natively</h3>
								<p className="mt-2 text-text-secondary max-w-md mx-auto">
									Use{" "}
									<code className="font-mono text-sm text-accent bg-border rounded px-1 py-0.5">c.broadcast</code> to
									send events. No external pub/sub or WebSocket server needed. It's built-in.
								</p>
							</div>

							{/* Add extra space at the end */}
							<div className="h-[10vh]"></div>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
}
