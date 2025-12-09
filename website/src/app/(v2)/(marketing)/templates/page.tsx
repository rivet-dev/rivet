import type { Metadata } from "next";
import TemplatesPageClient from "./TemplatesPageClient";

export const metadata: Metadata = {
	title: "Templates - Rivet",
	description:
		"Explore Rivet templates and examples to quickly start building with Rivet Actors. Find templates for AI agents, real-time apps, games, and more.",
	alternates: {
		canonical: "https://www.rivet.dev/templates/",
	},
};

export default function Page() {
	return <TemplatesPageClient />;
}
