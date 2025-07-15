import { Header } from "@/components/v2/Header";

export default function Layout({ children }) {
	return (
		<>
			<Header active="blog" variant="floating" />
			{children}
		</>
	);
}
