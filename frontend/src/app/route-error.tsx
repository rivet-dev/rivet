import { type ErrorComponentProps, Link } from "@tanstack/react-router";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	describeError,
} from "@/components";
import { RouteLayout } from "./route-layout";

export const RouteError = ({ error }: ErrorComponentProps) => {
	const { title, description } = describeError(error);
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						<Card>
							<CardHeader>
								<CardTitle className="flex items-center">
									{title}
								</CardTitle>
								<CardDescription>{description}</CardDescription>
							</CardHeader>
							<CardContent>
								<Button asChild variant="secondary">
									<Link to="." reloadDocument>
										Retry
									</Link>
								</Button>
							</CardContent>
						</Card>
					</div>
				</div>
			</div>
		</RouteLayout>
	);
};
