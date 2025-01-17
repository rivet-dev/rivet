"use client";
import SalesFramer from "@/generated/framer/sales";

import { usePostHog } from "posthog-js/react";

// This file is only used for `use client` directive

export const FramerSalesPage = () => {
	const posthog = usePostHog();
	return (
		<SalesFramer.Responsive
			style={{ width: "100%", background: "#000000" }}
			/* @ts-ignore */
			onSubmit={(event) => {
				event.preventDefault();
				const formData = new FormData(event.target);

				const data = Object.fromEntries(
					formData
						.values()
						.toArray()
						.map((value, index) => [
							`$survey_response${index > 0 ? `_${index}` : ""}`,
							value.toString(),
						]),
				);

				posthog.capture("survey sent", {
					$survey_id: "0193928a-4799-0000-8fc4-455382e21359",
					...data,
				});
			}}
			variants={{
				xl: "Desktop",
				md: "Tablet",
				sm: "Phone",
				base: "Phone",
			}}
		/>
	);
};
