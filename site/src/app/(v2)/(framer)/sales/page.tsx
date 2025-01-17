import { FramerSalesPage } from "@/components/framer/SalesPage";
import { FancyHeader } from "@/components/v2/FancyHeader";

export default function SalesPage() {
	// an empty div at the top of the page is a workaround for a bug in Next.js that causes the page to jump when the user navigates to it
	// https://github.com/vercel/next.js/discussions/64534
	return (
		<>
			<div />
			<FancyHeader />
			<div className="bg-black pb-20 pt-32">
				<FramerSalesPage />
			</div>
		</>
	);
}
