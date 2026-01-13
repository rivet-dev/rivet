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
			<p className="mt-4 text-lg text-white/60 text-center">
				<span className="text-2xl text-white mb-2 block">
					Thank you for your interest!
				</span>
				We will get back to you promptly. In the meantime, feel free to
				explore our{" "}
				<a href="/docs" className="text-[#FF5C00] hover:underline">
					documentation
				</a>{" "}
				or{" "}
				<a href="/blog" className="text-[#FF5C00] hover:underline">
					blog
				</a>{" "}
				for more information.
			</p>
		);
	}

	return (
		<form
			action="#"
			method="POST"
			className="mx-auto mt-16 max-w-xl sm:mt-20"
			onSubmit={handleSubmit}
		>
			<div className="grid grid-cols-1 gap-x-8 gap-y-6 sm:grid-cols-2">
				<div className="sm:col-span-2">
					<label
						htmlFor="email"
						className="block text-sm/6 font-semibold text-white"
					>
						Email
					</label>
					<div className="mt-2.5">
						<input
							id="email"
							name="$survey_response_0417ebe5-969d-41a9-8150-f702c42681ff"
							type="email"
							autoComplete="email"
							required
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="company"
						className="block text-sm/6 font-semibold text-white"
					>
						Company
					</label>
					<div className="mt-2.5">
						<input
							id="company"
							name="$survey_response_74c3d31a-880f-4e89-8cac-e03ad3422cce"
							type="text"
							autoComplete="organization"
							required
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="role"
						className="block text-sm/6 font-semibold text-white"
					>
						Role
					</label>
					<div className="mt-2.5">
						<input
							id="role"
							name="$survey_response_8bbdb054-6679-4d05-9685-f9f50d7b080b"
							type="text"
							required
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
							placeholder="e.g., CTO, Lead Engineer, Software Developer"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="current-stack"
						className="block text-sm/6 font-semibold text-white"
					>
						Current Stack
					</label>
					<div className="mt-2.5">
						<textarea
							id="current-stack"
							name="$survey_response_f585f0b9-f680-4b28-87f7-0d8f08fd0b14"
							rows={3}
							required
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
							placeholder="Tell us about your current technology stack and infrastructure"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="what-to-talk-about"
						className="block text-sm/6 font-semibold text-white"
					>
						What do you want to talk about?
					</label>
					<div className="mt-2.5">
						<textarea
							id="what-to-talk-about"
							name="$survey_response_3cdc5e4a-81f2-46e5-976b-15f8c2c8986f"
							rows={4}
							required
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
							placeholder="Describe your technical challenges, questions, or what you'd like to discuss with our engineer"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="where-heard"
						className="block text-sm/6 font-semibold text-white"
					>
						Where did you hear about us?
					</label>
					<div className="mt-2.5">
						<input
							id="where-heard"
							name="$survey_response_99519796-e67d-4a20-8ad4-ae5b7bb3e16d"
							type="text"
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
							placeholder="e.g., X, LinkedIn, Google, a colleague, etc."
						/>
					</div>
				</div>
			</div>
			<div className="mt-10">
				<button
					type="submit"
					className="w-full inline-flex items-center justify-center px-6 py-3 text-base font-medium rounded-xl transition-all duration-200 active:scale-[0.97] bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white"
					disabled={isSubmitting}
				>
					Let's talk
				</button>
			</div>
		</form>
	);
}
