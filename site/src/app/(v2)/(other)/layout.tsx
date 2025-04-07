import { FancyHeader } from "@/components/v2/FancyHeader";

export default function Layout({ children }: { children: React.ReactNode }) {
	return (
		<>
			<FancyHeader />
			{children}
		</>
	);
}
