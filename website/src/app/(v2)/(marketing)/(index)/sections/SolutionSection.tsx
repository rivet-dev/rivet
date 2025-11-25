"use client";

import { useEffect } from "react";

export function SolutionSection() {
	useEffect(() => {
		// Sticky code highlighting with scroll-based step animations
		const codeSteps = document.querySelectorAll<HTMLDivElement>(".code-highlight-step");
		if (codeSteps.length === 0) return;

		const sectionTrigger = document.querySelector<HTMLDivElement>(".section-scale-trigger");
		const codeBox = document.querySelector<HTMLDivElement>(".code-box-container");
		const codePre = document.querySelector<HTMLElement>(".code-content pre");
		const sectionHeader = document.querySelector<HTMLHeadingElement>(".section-header");
		const codeRefs = new Map(
			Array.from(document.querySelectorAll<HTMLElement>(".code-highlight-ref")).map((el) => [el.id, el])
		);
		const allHighlightRefs = document.querySelectorAll<HTMLElement>(".code-highlight-ref");
		let activeCodeRef: HTMLElement | null = null;
		let activeStep: HTMLElement | null = null;

		// Track state
		let isInExpandedZone = false; // Are we in the zone where box should be 1.2?
		let currentActiveStepIndex = -1; // Which step is currently active

		// Helper function to apply scale
		const applyScale = (shouldExpand: boolean) => {
			if (!codeBox) return;

			if (shouldExpand) {
				codeBox.style.transform = "scale(1.20)";
				codeBox.style.transition = "transform 0.5s ease-in-out";

				if (codePre) {
					codePre.style.transform = "scale(0.85)";
					codePre.style.transformOrigin = "0 0";
					codePre.style.transition = "transform 0.5s ease-in-out";
				}

				allHighlightRefs.forEach((ref) => {
					ref.style.marginLeft = "-1.8rem";
					ref.style.marginRight = "0";
					ref.style.paddingLeft = "2.5rem";
					ref.style.paddingRight = "2.5rem";
					ref.style.width = "calc((100% / 0.85) + 1.8rem + 1.5rem + 0.235rem)";
					ref.style.transition = "margin 0.5s ease-in-out, padding 0.5s ease-in-out, width 0.5s ease-in-out";
				});

				if (sectionHeader) {
					sectionHeader.style.marginBottom = "4.8rem"; // 4rem * 1.2 to maintain constant visual spacing
					sectionHeader.style.transition = "margin-bottom 0.5s ease-in-out";
				}
			} else {
				codeBox.style.transform = "scale(1)";
				codeBox.style.transition = "transform 0.5s ease-in-out";

				if (codePre) {
					codePre.style.transform = "scale(1)";
					codePre.style.transformOrigin = "";
					codePre.style.transition = "transform 0.5s ease-in-out";
				}

				allHighlightRefs.forEach((ref) => {
					ref.style.marginLeft = "";
					ref.style.marginRight = "";
					ref.style.paddingLeft = "";
					ref.style.paddingRight = "";
					ref.style.width = "";
					ref.style.transition = "margin 0.5s ease-in-out, padding 0.5s ease-in-out, width 0.5s ease-in-out";
				});

				if (sectionHeader) {
					sectionHeader.style.marginBottom = "";
					sectionHeader.style.transition = "margin-bottom 0.5s ease-in-out";
				}
			}
		};

		// Helper to determine if box should be expanded
		const updateScale = () => {
			// Expand if: we're in the expanded zone AND active step is 0-3
			const shouldExpand = isInExpandedZone && currentActiveStepIndex >= 0 && currentActiveStepIndex <= 3;
			applyScale(shouldExpand);
		};

		// Observer for the section trigger - determines if we're in the "expanded zone"
		const triggerObserver = new IntersectionObserver(
			(entries) => {
				entries.forEach((entry) => {
					isInExpandedZone = entry.isIntersecting;
					updateScale();
				});
			},
			{
				rootMargin: "-42% 0px -42% 0px",
				threshold: 0,
			}
		);

		// Observer for steps - determines which step is active
		const stepObserver = new IntersectionObserver(
			(entries) => {
				entries.forEach((entry) => {
					const stepEl = entry.target as HTMLElement;
					const stepIndex = Array.from(codeSteps).indexOf(stepEl as HTMLDivElement);
					const targetId = stepEl.dataset.targetCode;
					const codeEl = codeRefs.get(targetId || "");
					const isLastStep = stepIndex === codeSteps.length - 1;

					if (entry.isIntersecting) {
						// This step is now active
						currentActiveStepIndex = stepIndex;

						// Hide all other steps
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

						// Activate code highlighting
						if (activeCodeRef && activeCodeRef !== codeEl) {
							activeCodeRef.classList.remove("is-active");
						}
						if (codeEl) {
							codeEl.classList.add("is-active");
							activeCodeRef = codeEl;
						}

						// Show step
						stepEl.style.opacity = "1";
						stepEl.style.transform = "translateY(0) scale(1)";
						activeStep = stepEl;

						// Update scale based on new active step
						updateScale();
					} else {
						// Special handling for last step
						if (isLastStep) {
							const rect = stepEl.getBoundingClientRect();
							const hasScrolledPast = rect.top < window.innerHeight / 2;
							if (hasScrolledPast) {
								currentActiveStepIndex = stepIndex;
								stepEl.style.opacity = "1";
								stepEl.style.transform = "translateY(0) scale(1)";
								if (codeEl && !codeEl.classList.contains("is-active")) {
									if (activeCodeRef) activeCodeRef.classList.remove("is-active");
									codeEl.classList.add("is-active");
									activeCodeRef = codeEl;
								}
								activeStep = stepEl;
								updateScale();
								return;
							}
						}

						// Hide step
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

		// Observe the trigger element
		if (sectionTrigger) {
			triggerObserver.observe(sectionTrigger);
		}

		// Observe all steps
		codeSteps.forEach((step) => {
			step.style.transition = "opacity 0.35s ease-in-out, transform 0.35s ease-in-out";
			stepObserver.observe(step);
		});

		return () => {
			if (sectionTrigger) {
				triggerObserver.unobserve(sectionTrigger);
			}
			codeSteps.forEach((step) => stepObserver.unobserve(step));
		};
	}, []);

	return (
		<section className="relative pt-48 md:pt-64">
			{/* Centered Container */}
			<div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
				{/* Header */}
				<h2 className="section-header text-center font-heading text-4xl md:text-5xl font-bold tracking-tighter text-text-primary mb-16">
					Stop Writing Glue Code.
				</h2>

				{/* Content Grid */}
				<div className="grid grid-cols-1 md:grid-cols-2 gap-8 md:gap-16">
					{/* Left Side: Sticky Code */}
					<div>
						<div className="sticky top-32 md:top-40 z-10">
							<div className="code-box-container flex-none rounded-xl border border-border bg-black relative overflow-hidden" style={{ transformOrigin: "center center" }}>
								{/* Top Shine Highlight */}
								<div className="absolute top-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-white/30 to-transparent z-10" />
								<div className="flex items-center justify-between border-b border-border px-4 py-3">
									<span className="text-sm font-medium text-text-primary">ai-agent.ts</span>
									<span className="text-sm text-text-secondary font-mono">actor.ts</span>
								</div>
								<div className="code-content p-6 font-mono text-sm overflow-x-auto overflow-y-visible [scrollbar-width:none] [&::-webkit-scrollbar]:hidden bg-black" style={{ transformOrigin: "center center" }}>
								<pre className="bg-black">
									<code className="bg-black">
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
						<div className="flex flex-col items-center relative">
							{/* Invisible trigger element - spans from before step 1 through step 4 */}
							{/* This keeps isInExpandedZone true throughout steps 1-4 */}
							<div className="section-scale-trigger absolute top-0 left-0 right-0" style={{ height: "calc(10vh + 20vh + 20vh + 20vh + 40vh)", pointerEvents: "none" }}></div>

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
