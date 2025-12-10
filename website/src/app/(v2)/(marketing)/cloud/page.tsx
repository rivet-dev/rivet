import type { Metadata } from "next";
import PricingPageClient from "../pricing/PricingPageClient";

export const metadata: Metadata = {
	title: "Cloud - Rivet",
	description:
		"Rivet Cloud—managed cloud solution for stateful backends. Transparent pricing, global infrastructure, and seamless orchestration.",
	alternates: {
		canonical: "https://www.rivet.dev/cloud/",
	},
};

export default function Page() {
	return <PricingPageClient />;
}

