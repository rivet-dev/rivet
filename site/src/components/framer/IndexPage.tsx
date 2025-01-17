"use client";
import "@/generated/framer/styles.css";
import "@/generated/framer/tokens.css";
import IndexFramer from "@/generated/framer/index";

// This file is only used for `use client` directive

export const FramerIndexPage = () => {
	return (
		<IndexFramer.Responsive
			style={{ width: "100%", background: "#000000" }}
			variants={{
				xl: "Desktop",
				md: "Tablet",
				sm: "Phone",
				base: "Phone",
			}}
		/>
	);
};
