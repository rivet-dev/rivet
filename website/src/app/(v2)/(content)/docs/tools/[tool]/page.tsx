import { redirect } from "next/navigation";

interface PageProps {
	params: Promise<{
		tool: string;
	}>;
}

export default async function Page({ params }: PageProps) {
	const { tool } = await params;
	// HACK: This page allows us to put tools in the sidebar but redirect to a different page. We can't use `href` since that will change which sidebar/tab is active when on the tool's page.
	redirect(`/docs/${tool}`);
}

export function generateStaticParams() {
	return [{ tool: "actors" }];
}
