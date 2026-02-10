"use client";

import posthog from "posthog-js";
import { useState } from "react";

export function SalesForm() {
	const [isSubmitted, setIsSubmitted] = useState(false);
	const [isSubmitting, setIsSubmitting] = useState(false);

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSubmitting) return;

		setIsSubmitting(true);

		const formData = new FormData(event.currentTarget);
		const data = Object.fromEntries(formData.entries().toArray());

		try {
			posthog.capture("survey sent", {
				$survey_id: "0193928a-4799-0000-8fc4-455382e21359",
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
					We will get back to you within the next few days. In the meantime, feel
					free to explore our{" "}
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
			<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<div>
					<label htmlFor="first-name" className={labelClasses}>
						First name
					</label>
					<input
						id="first-name"
						name="$survey_response_27cd441e-3b34-4ea3-b2cf-e22b847046d9"
						type="text"
						autoComplete="given-name"
						className={inputClasses}
					/>
				</div>
				<div>
					<label htmlFor="last-name" className={labelClasses}>
						Last name
					</label>
					<input
						id="last-name"
						name="$survey_response_effaa684-34bb-468e-80fb-29b693d564d5"
						type="text"
						autoComplete="family-name"
						className={inputClasses}
					/>
				</div>
				<div className="sm:col-span-2">
					<label htmlFor="company" className={labelClasses}>
						Company
					</label>
					<input
						id="company"
						name="$survey_response_feaa095f-16a0-47f0-871d-361d2a446c2c"
						type="text"
						autoComplete="organization"
						className={inputClasses}
					/>
				</div>
				<div className="sm:col-span-2">
					<label htmlFor="email" className={labelClasses}>
						Email
					</label>
					<input
						id="email"
						name="$survey_response_c954c48d-b373-475e-8eb5-0023ed18182b"
						type="email"
						autoComplete="email"
						className={inputClasses}
					/>
				</div>
				<div className="sm:col-span-2">
					<label htmlFor="message" className={labelClasses}>
						Message
					</label>
					<textarea
						id="message"
						name="$survey_response_8974a198-041d-494b-945e-d829d192be2b"
						rows={4}
						className={inputClasses}
						placeholder="I would like Rivet to help solve for my company..."
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
