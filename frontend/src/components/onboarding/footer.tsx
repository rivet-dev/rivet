import { faArrowUpRight, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { match } from "ts-pattern";
import { Button } from "../ui/button";

export function OnboardingFooter() {
	return (
		<div className="flex gap-4 justify-center py-8 bg-gradient-to-t from-background to-transparent">
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://rivet.dev/docs"
					target="_blank"
					rel="noopener noreferrer"
				>
					Documentation
				</a>
			</Button>
			{match(__APP_TYPE__)
				.with("cloud", () => (
					<Button
						className="text-muted-foreground"
						variant="link"
						size="xs"
						onClick={() => {
							Plain.open();
						}}
						endIcon={
							<Icon icon={faArrowUpRight} className="ms-1" />
						}
					>
						Support
					</Button>
				))
				.otherwise(() => (
					<Button
						className="text-muted-foreground"
						variant="link"
						size="xs"
						asChild
						endIcon={
							<Icon icon={faArrowUpRight} className="ms-1" />
						}
					>
						<Link
							to="."
							search={(old) => ({
								...old,
								modal: "feedback",
							})}
						>
							Feedback
						</Link>
					</Button>
				))}
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://rivet.gg/discord"
					target="_blank"
					rel="noopener noreferrer"
				>
					Discord
				</a>
			</Button>
			<Button
				variant="link"
				size="xs"
				className="text-muted-foreground"
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
			>
				<a
					href="http://github.com/rivet-gg/rivet"
					target="_blank"
					rel="noopener noreferrer"
				>
					GitHub
				</a>
			</Button>
		</div>
	);
}
