import { FramerIndexPage } from "@/components/framer/IndexPage";
import { FancyHeader } from "@/components/v2/FancyHeader";
import type { Metadata } from "next";

export const metadata: Metadata = {
	description: "Run and scale realtime applications",
};

export default function IndexPage() {
	// an empty div at the top of the page is a workaround for a bug in Next.js that causes the page to jump when the user navigates to it
	// https://github.com/vercel/next.js/discussions/64534
	return (
		<>
			<div />
			<FancyHeader />
			<FramerIndexPage />
		</>
	);
}
