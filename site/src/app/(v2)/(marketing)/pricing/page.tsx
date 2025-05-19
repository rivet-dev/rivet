import type { Metadata } from "next";
import PricingPageClient from "./PricingPageClient";

export const metadata: Metadata = {
	title: "Pricing - Rivet",
	description: "Simple, transparent pricing for all your serverless needs",
};

export default function Page() {
	return <PricingPageClient />;
}
