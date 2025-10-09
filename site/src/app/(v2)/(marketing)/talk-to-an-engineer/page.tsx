import type { Metadata } from "next";
import TalkToAnEngineerPageClient from "./TalkToAnEngineerPageClient";

export const metadata: Metadata = {
	title: "Talk to an Engineer - Rivet",
	description:
		"Connect with a Rivet engineer to discuss your technical needs, current stack, and how we can help with your infrastructure challenges",
	alternates: {
		canonical: "https://www.rivet.dev/talk-to-an-engineer/",
	},
};

export default function TalkToAnEngineerPage() {
	return <TalkToAnEngineerPageClient />;
}