"use client";
import PricingFramer from "@/generated/framer/pricing";

// This file is only used for `use client` directive

export const FramerPricingPage = () => {
	return (
		<PricingFramer.Responsive
			style={{ width: "100%", background: "#000000" }}
			variants={{
				xl: "Desktop",
				md: "Tablet",
				sm: "Phone",
				base: "Phone",
			}}
		/>
	);
};
