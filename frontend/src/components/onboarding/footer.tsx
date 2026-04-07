import { faArrowUpRight, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
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
					href="https://rivet.dev/docs"
					target="_blank"
					rel="noopener noreferrer"
				>
					Documentation
				</a>
			</Button>
			<Button
				className="text-muted-foreground"
				variant="link"
				size="xs"
				asChild
				endIcon={<Icon icon={faArrowUpRight} className="ms-1" />}
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
					href="https://github.com/rivet-dev/rivet"
					target="_blank"
					rel="noopener noreferrer"
				>
					GitHub
				</a>
			</Button>
		</div>
	);
}
