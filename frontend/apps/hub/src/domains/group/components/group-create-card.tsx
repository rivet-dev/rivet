import { Button, Card, CardFooter } from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";

export function GroupCreateCard() {
	return (
		<>
			<Card w="full" my="4">
				<CardFooter>
					<Button asChild variant="secondary">
						<Link to="/" search={{ modal: "create-group" }}>
							Create a new team
						</Link>
					</Button>
				</CardFooter>
			</Card>
		</>
	);
}
