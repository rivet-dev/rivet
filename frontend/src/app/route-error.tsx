import { type ErrorComponentProps, Link } from "@tanstack/react-router";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components";
import { RouteLayout } from "./route-layout";

export const RouteError = ({ error }: ErrorComponentProps) => {
	return (
		<RouteLayout>
			<div className="bg-card h-full border my-2 mr-2 rounded-lg">
				<div className="mt-2 flex flex-col items-center justify-center h-full">
					<div className="w-full sm:w-96">
						<Card>
							<CardHeader>
								<CardTitle className="flex items-center">
									{"statusCode" in error &&
									error.statusCode === 404
										? "Resource not found"
										: "body" in error &&
												error.body &&
												typeof error.body ===
													"object" &&
												"message" in error.body
											? String(error.body.message)
											: error.message}
								</CardTitle>
								<CardDescription>
									{"statusCode" in error &&
									error.statusCode === 404
										? "The resource you are looking for does not exist or you do not have access to it."
										: "An unexpected error occurred. Please try again later."}
								</CardDescription>
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
