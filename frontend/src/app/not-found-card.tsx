import { Link } from "@tanstack/react-router";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components";

export function NotFoundCard() {
	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="mt-2 flex flex-col items-center justify-center h-full">
				<div className="w-full sm:w-96">
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center">
								404
							</CardTitle>
							<CardDescription>
								The page was not found
							</CardDescription>
						</CardHeader>
						<CardContent>
							<Button asChild variant="secondary">
								<Link to="/">Go home</Link>
							</Button>
						</CardContent>
					</Card>
				</div>
			</div>
		</div>
	);
}
