import { EmbedDetector } from "@/components/EmbedDetector";
import { Footer } from "@/components/Footer";
import "@/styles/v2.css";
import { Suspense } from "react";

export default function Layout({ children }) {
	return (
		<>
			{children}
			<Suspense>
				<EmbedDetector />
			</Suspense>
			<Footer />
		</>
	);
}
