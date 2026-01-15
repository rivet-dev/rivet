import WorkflowsPage from "@/components/marketing/solutions/WorkflowsPage";
import type { Metadata } from "next";

export const metadata: Metadata = {
	title: "Workflows - Rivet",
	description: "Build durable workflows with Rivet. Long-running processes, automatic retries, and reliable state management.",
	alternates: {
		canonical: "https://rivet.gg/solutions/workflows/",
	},
};

export default function Page() {
	return <WorkflowsPage />;
}
