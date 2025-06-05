"use client";

import posthog from "posthog-js";
import { useState } from "react";

export function SalesForm() {
	const [isSubmitted, setIsSubmitted] = useState(false);

	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		const formData = new FormData(event.currentTarget);

		const data = Object.fromEntries(formData.entries().toArray());

		console.log(data);

		posthog.capture("survey sent", {
			$survey_id: "0193928a-4799-0000-8fc4-455382e21359",
			...data,
		});
		setIsSubmitted(true);
	};

	if (isSubmitted) {
		return (
			<p className="mt-4 text-lg text-white/60 text-center">
				<span className="text-2xl text-white mb-2 block">
					Thank you for your interest!
				</span>
				We will get back to you in within the next few days. In the
				meantime, feel free to explore our{" "}
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
				<div>
					<label
						htmlFor="first-name"
						className="block text-sm/6 font-semibold text-white"
					>
						First name
					</label>
					<div className="mt-2.5">
						<input
							id="first-name"
							name="$survey_response_3"
							type="text"
							autoComplete="given-name"
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
						/>
					</div>
				</div>
				<div>
					<label
						htmlFor="last-name"
						className="block text-sm/6 font-semibold text-white"
					>
						Last name
					</label>
					<div className="mt-2.5">
						<input
							id="last-name"
							name="$survey_response_4"
							type="text"
							autoComplete="family-name"
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
							name="$survey_response_2"
							type="text"
							autoComplete="organization"
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
						/>
					</div>
				</div>
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
							name="$survey_response"
							type="email"
							autoComplete="email"
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
						/>
					</div>
				</div>
				<div className="sm:col-span-2">
					<label
						htmlFor="message"
						className="block text-sm/6 font-semibold text-white"
					>
						Message
					</label>
					<div className="mt-2.5">
						<textarea
							id="message"
							name="$survey_response_1"
							rows={4}
							className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
							placeholder="I would like Rivet to help solve for my company..."
						/>
					</div>
				</div>
			</div>
			<div className="mt-10">
				<button
					type="submit"
					className="w-full inline-flex items-center justify-center px-6 py-3 text-base font-medium rounded-xl transition-all duration-200 active:scale-[0.97] bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white"
				>
					Let's talk
				</button>
			</div>
		</form>
	);
}
