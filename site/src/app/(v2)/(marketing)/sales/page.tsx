import type { Metadata } from "next";
import SalesPageClient from "./SalesPageClient";

export const metadata: Metadata = {
	title: "Enterprise Sales - Rivet",
	description: "Contact Rivet to discuss enterprise-grade serverless infrastructure for AI agents, realtime systems, and scalable function-based workloads",
};

export default function SalesPage() {
	return <SalesPageClient />;
} 