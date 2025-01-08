import { CommandGroup, CommandItem } from "@rivet-gg/components";
import {
	Icon,
	faBook,
	faComment,
	faLifeRing,
	faMessageHeart,
} from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";

export function RivetCommandGroup() {
	const navigate = useNavigate();
	return (
		<CommandGroup heading="Rivet">
			<CommandItem
				onSelect={() => window.open("https://rivet.gg/docs", "_blank")}
			>
				<Icon icon={faBook} />
				Docs
			</CommandItem>

			<CommandItem
				onSelect={() =>
					navigate({ to: ".", search: { modal: "feedback" } })
				}
			>
				<Icon icon={faComment} />
				Feedback
			</CommandItem>
			<CommandItem
				onSelect={() =>
					window.open("https://rivet.gg/support", "_blank")
				}
			>
				<Icon icon={faLifeRing} />
				Support
			</CommandItem>
			<CommandItem
				onSelect={() =>
					window.open("https://rivet.gg/discord", "_blank")
				}
			>
				<Icon icon={faMessageHeart} />
				Discord
			</CommandItem>
		</CommandGroup>
	);
}
