import { Header } from "@/components/v2/Header";
import type { CSSProperties } from "react";

export default function Layout({ children }) {
	return (
		<>
			<Header active="blog" variant="floating" />
			<div
				className="mx-auto mt-20 w-full max-w-6xl px-8 md:mt-32"
				style={{ "--header-height": "5rem" } as CSSProperties}
			>
				{children}
			</div>
		</>
	);
}
