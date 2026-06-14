"use client";

import posthog from "posthog-js";
import * as Sentry from "@sentry/astro"
import { useState } from "react";

const fields = {
	email: '$survey_response_0417ebe5-969d-41a9-8150-f702c42681ff',
	company: '$survey_response_74c3d31a-880f-4e89-8cac-e03ad3422cce',
	role: "$survey_response_8bbdb054-6679-4d05-9685-f9f50d7b080b",
	currentStack: "$survey_response_f585f0b9-f680-4b28-87f7-0d8f08fd0b14",
	whatToTalkAbout: "$survey_response_3cdc5e4a-81f2-46e5-976b-15f8c2c8986f",
	whereHeard: "$survey_response_99519796-e67d-4a20-8ad4-ae5b7bb3e16d",
}

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
			Sentry.captureFeedback({
				message: `Role: ${data[fields.role] as string}\nCurrent Stack: ${data[fields.currentStack] as string}\nWhat to Talk About: ${data[fields.whatToTalkAbout] as string}`,
				email: data[fields.email] as string,
				tags: {
					company: data[fields.company] as string,
				}
			})
			setIsSubmitted(true);
		} finally {
			setIsSubmitting(false);
		}
	};

	if (isSubmitted) {
		return (
			<div className="mt-8 text-center">
				<p className="text-2xl font-medium text-ink mb-4">
					Thank you for your interest!
				</p>
				<p className="text-ink-soft">
					We will get back to you promptly. In the meantime, feel free to
					explore our{" "}
					<a href="/docs" className="text-pine hover:text-ink underline underline-offset-2">
						documentation
					</a>{" "}
					or{" "}
					<a href="/changelog" className="text-pine hover:text-ink underline underline-offset-2">
						changelog
					</a>{" "}
					for more information.
				</p>
			</div>
		);
	}

	const inputClasses = "block w-full rounded-md border border-ink/15 bg-white/55 px-3 py-2 text-sm text-ink placeholder:text-ink-faint focus:border-pine focus:outline-none transition-colors";
	const labelClasses = "block text-sm font-medium text-ink-soft mb-1.5";

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
						name={fields.email}
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
						name={fields.company}
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
						name={fields.role}
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
						name={fields.currentStack}
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
						name={fields.whatToTalkAbout}
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
						name={fields.whereHeard}
						type="text"
						className={inputClasses}
						placeholder="e.g., X, LinkedIn, Google, a colleague, etc."
					/>
				</div>
			</div>
			<div className="mt-6 text-center">
				<button
					type="submit"
					className="inline-flex items-center justify-center gap-2 rounded-md bg-accent-deep px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
					disabled={isSubmitting}
				>
					{isSubmitting ? "Submitting..." : "Let's talk"}
				</button>
			</div>
		</form>
	);
}
