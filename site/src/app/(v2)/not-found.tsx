import { Button } from "@/components/Button";
import { Header } from "@/components/v2/Header";
import { faBlockQuestion, Icon } from "@rivet-gg/icons";

export default function NotFound() {
	return (
		<>
			<Header variant="floating" />
			<div className="relative flex min-h-[80vh] w-full items-center justify-center text-center">
				<div className="transition-opacity">
					<h1 className="mb-3 flex items-center justify-center text-3xl text-white">
						<Icon className="mr-2 size-10" icon={faBlockQuestion} />{" "}
						Not Found
					</h1>
					<Button href="/" variant="secondary">
						Home
					</Button>
				</div>
			</div>
		</>
	);
}
