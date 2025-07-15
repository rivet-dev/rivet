import type { Metadata } from "next";
import TalkToAnEngineerPageClient from "./TalkToAnEngineerPageClient";

export const metadata: Metadata = {
	title: "Talk to an Engineer - Rivet",
	description:
		"Connect with a Rivet engineer to discuss your technical needs, current stack, and how we can help with your infrastructure challenges",
};

export default function TalkToAnEngineerPage() {
	return <TalkToAnEngineerPageClient />;
}