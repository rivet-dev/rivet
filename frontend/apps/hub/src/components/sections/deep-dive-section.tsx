import { Button, Grid, H2 } from "@rivet-gg/components";
import { Icon, faArrowRight } from "@rivet-gg/icons";

export function DeepDiveSection() {
	return (
		<>
			<H2>Deep dive</H2>
			<Grid columns={{ initial: "1", md: "2" }} gap="4">
				<Button
					variant="outline"
					className="flex h-full justify-between items-center text-left min-h-20"
					endIcon={<Icon icon={faArrowRight} />}
					asChild
				>
					<a
						href="https://github.com/rivet-gg/examples"
						target="_blank"
						rel="noreferrer"
					>
						<span className="font-bold text-2xl">Quick start</span>
					</a>
				</Button>
				<Button
					variant="outline"
					className="flex h-full justify-between items-center text-left  min-h-20"
					endIcon={<Icon icon={faArrowRight} />}
					asChild
				>
					<a
						href="https://rivet.gg/docs"
						target="_blank"
						rel="noreferrer"
					>
						<span className="font-bold text-2xl">Docs</span>
					</a>
				</Button>
				<Button
					variant="outline"
					className="flex h-full justify-between items-center text-left  min-h-20"
					endIcon={<Icon icon={faArrowRight} />}
					asChild
				>
					<a
						href="https://rivet.gg/discord"
						target="_blank"
						rel="noreferrer"
					>
						<span className="font-bold text-2xl">Discord</span>
					</a>
				</Button>
				<Button
					variant="outline"
					className="flex h-full justify-between items-center text-left  min-h-20"
					endIcon={<Icon icon={faArrowRight} />}
					asChild
				>
					<a
						href="https://github.com/rivet-gg"
						target="_blank"
						rel="noreferrer"
					>
						<span className="font-bold text-2xl">GitHub</span>
					</a>
				</Button>
			</Grid>
		</>
	);
}
