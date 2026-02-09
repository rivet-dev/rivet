"use client";

import posthog from "posthog-js";
import { useState } from "react";

export function TalkToAnEngineerForm() {
	const [isSubmitted, setIsSubmitted] = useState(false);
	const [isSubmitting, setIsSubmitting] = useState(false);

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSubmitting) return;

		setIsSubmitting(true);

		const formData = new FormData(event.currentTarget);

		const data = Object.fromEntries(formData.entries().toArray());

		console.log(data);

		try {
			posthog.capture("survey sent", {
				$survey_id: "01980f18-06a9-0000-e1e1-a5886e9012d0",
				...data,
			});
			setIsSubmitted(true);
		} finally {
			setIsSubmitting(false);
		}
	};

	if (isSubmitted) {
		return (
			<div className="mt-8 text-center">
				<p className="text-2xl font-normal text-white mb-4">
					Thank you for your interest!
				</p>
				<p className="text-zinc-400">
					We will get back to you promptly. In the meantime, feel free to
					explore our{" "}
					<a href="/docs" className="text-white hover:text-zinc-300 underline underline-offset-2">
						documentation
					</a>{" "}
					or{" "}
					<a href="/changelog" className="text-white hover:text-zinc-300 underline underline-offset-2">
						changelog
					</a>{" "}
					for more information.
				</p>
			</div>
		);
	}

	const inputClasses = "block w-full rounded-md border border-white/10 bg-black px-3 py-2 text-sm text-white placeholder:text-zinc-600 focus:border-white/20 focus:outline-none transition-colors";
	const labelClasses = "block text-sm font-medium text-zinc-400 mb-1.5";

	return (
		<form
			action="#"
			method="POST"
			onSubmit={handleSubmit}
		>
			<div className="flex flex-col gap-4">
				<div>
					<label htmlFor="email" className={labelClasses}>
						Email
					</label>
					<input
						id="email"
						name="$survey_response_0417ebe5-969d-41a9-8150-f702c42681ff"
						type="email"
						autoComplete="email"
						required
						className={inputClasses}
					/>
				</div>
				<div>
					<label htmlFor="company" className={labelClasses}>
						Company
					</label>
					<input
						id="company"
						name="$survey_response_74c3d31a-880f-4e89-8cac-e03ad3422cce"
						type="text"
						autoComplete="organization"
						required
						className={inputClasses}
					/>
				</div>
				<div>
					<label htmlFor="role" className={labelClasses}>
						Role
					</label>
					<input
						id="role"
						name="$survey_response_8bbdb054-6679-4d05-9685-f9f50d7b080b"
						type="text"
						required
						className={inputClasses}
						placeholder="e.g., CTO, Lead Engineer, Software Developer"
					/>
				</div>
				<div>
					<label htmlFor="current-stack" className={labelClasses}>
						Current Stack
					</label>
					<textarea
						id="current-stack"
						name="$survey_response_f585f0b9-f680-4b28-87f7-0d8f08fd0b14"
						rows={3}
						required
						className={inputClasses}
						placeholder="Tell us about your current technology stack and infrastructure"
					/>
				</div>
				<div>
					<label htmlFor="what-to-talk-about" className={labelClasses}>
						What do you want to talk about?
					</label>
					<textarea
						id="what-to-talk-about"
						name="$survey_response_3cdc5e4a-81f2-46e5-976b-15f8c2c8986f"
						rows={4}
						required
						className={inputClasses}
						placeholder="Describe your technical challenges, questions, or what you'd like to discuss with our engineer"
					/>
				</div>
				<div>
					<label htmlFor="where-heard" className={labelClasses}>
						Where did you hear about us?
					</label>
					<input
						id="where-heard"
						name="$survey_response_99519796-e67d-4a20-8ad4-ae5b7bb3e16d"
						type="text"
						className={inputClasses}
						placeholder="e.g., X, LinkedIn, Google, a colleague, etc."
					/>
				</div>
			</div>
			<div className="mt-6 text-center">
				<button
					type="submit"
					className="inline-flex items-center justify-center gap-2 rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200 disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={isSubmitting}
				>
					{isSubmitting ? "Submitting..." : "Let's talk"}
				</button>
			</div>
		</form>
	);
}
