import CollaborativeStatePage from "@/components/marketing/solutions/CollaborativeStatePage";
import type { Metadata } from "next";

export const metadata: Metadata = {
	title: "Collaborative State - Rivet",
	description: "Build real-time collaborative applications with Rivet Actors. Shared state, conflict resolution, and instant synchronization.",
	alternates: {
		canonical: "https://rivet.gg/solutions/collaborative-state/",
	},
};

export default function Page() {
	return <CollaborativeStatePage />;
}
