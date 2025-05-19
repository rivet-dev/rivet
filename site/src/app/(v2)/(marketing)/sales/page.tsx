import type { Metadata } from "next";
import SalesPageClient from "./SalesPageClient";

export const metadata: Metadata = {
	title: "Enterprise Sales - Rivet",
	description: "Contact our sales team to learn more about Rivet Enterprise",
};

export default function SalesPage() {
	return <SalesPageClient />;
} 