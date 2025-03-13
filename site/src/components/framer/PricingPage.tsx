"use client";
import PricingFramer from "@/generated/framer/pricing";
import { useRouter } from "next/navigation";
import { UnframerProvider } from "unframer";

// This file is only used for `use client` directive

export const FramerPricingPage = () => {
	const router = useRouter();
	return (
		<UnframerProvider navigate={router.push}>
			<PricingFramer.Responsive
				style={{ width: "100%", background: "#000000" }}
				variants={{
					xl: "Desktop",
					md: "Tablet",
					sm: "Phone",
					base: "Phone",
				}}
			/>
		</UnframerProvider>
	);
};
