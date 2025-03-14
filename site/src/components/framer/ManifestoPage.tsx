"use client";
import ManifestoFramer from "@/generated/framer/manifesto";
import { useRouter } from "next/navigation";
import { UnframerProvider } from "unframer";

export const FramerManifestoPage = () => {
	const router = useRouter();
	return (
		<UnframerProvider navigate={router.push}>
			<ManifestoFramer.Responsive
				style={{ width: "100%", background: "#000000" }}
				variants={{
					xl: "Desktop",
					md: "Tablet",
					sm: "Phone",
					base: "Phone",
				}}
			/>
		</UnframerProvider>
	);
}; 