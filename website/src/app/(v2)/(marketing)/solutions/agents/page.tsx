import AgentsPage from "@/components/marketing/solutions/AgentsPage";
import type { Metadata } from "next";

export const metadata: Metadata = {
	title: "AI Agents - Rivet",
	description: "Build AI agents with stateful backends using Rivet Actors. Persistent memory, tool execution, and long-running context for intelligent agents.",
	alternates: {
		canonical: "https://rivet.gg/solutions/agents/",
	},
};

export default function Page() {
	return <AgentsPage />;
}
