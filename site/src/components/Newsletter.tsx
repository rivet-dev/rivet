"use client";

import posthog from "posthog-js";
import { Button, Input, Label } from "@rivet-gg/components";
import { useState } from "react";

export function Newsletter() {
	const [isSubmitted, setIsSubmitted] = useState(false);
	const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
		if (isSubmitted) return; // Prevent multiple submissions
		event.preventDefault();
		const formData = new FormData(event.currentTarget);

		const data = Object.fromEntries(formData.entries().toArray());

		posthog.capture("survey sent", {
			$survey_id: "01983e70-b743-0000-e4a7-07ce220da177",
			...data,
		});

		posthog.setPersonProperties({
			email: Object.values(data)[0],
		});
		setIsSubmitted(true);

		const form = event.currentTarget;

		setTimeout(() => {
			form.reset();
			setIsSubmitted(false);
		}, 3000);
	};

	return (
		<form className="mt-6 px-2" onSubmit={handleSubmit}>
			<div className="flex flex-col gap-2">
				<Label htmlFor="newsletter-email" className="text-sm">
					Rivet Newsletter
				</Label>
				<Input
					type="email"
					required
					placeholder="you@company.com"
					className="text-sm"
					autoComplete="email"
					name="$survey_response_2adad347-bc39-48f3-b5d1-755278685c94"
				/>
				<Button variant="secondary" type="submit">
					{isSubmitted ? "Subscribed" : "Subscribe"}
				</Button>
			</div>
		</form>
	);
}
