import { FancyHeader } from "@/components/v2/FancyHeader";

export default function Layout({ children }) {
	return (
		<>
			<FancyHeader active="blog" />
			{children}
		</>
	);
}
