import { Header } from "@/components/v2/Header";

export default function Layout({ children }: { children: React.ReactNode }) {
	return (
		<>
			<Header variant="floating" />
			{children}
		</>
	);
}
