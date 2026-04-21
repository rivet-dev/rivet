import { faMagnifyingGlass, Icon } from "@rivet-gg/icons";
import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { Button, type ButtonProps } from "@/components";
import { Input } from "@/components/ui/input";
import { useActorsView } from "./actors-view-context-provider";
import { useDataProvider } from "./data-provider";

export function GoToActorButton(props: ButtonProps) {
	const [open, setOpen] = useState(false);
	const [value, setValue] = useState("");
	const [isPending, setIsPending] = useState(false);
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const dataProvider = useDataProvider();
	const { copy } = useActorsView();

	const handleSubmit = async () => {
		const trimmed = value.trim();
		if (!trimmed) {
			setOpen(false);
			return;
		}

		setIsPending(true);
		try {
			await queryClient.fetchQuery(
				dataProvider.actorQueryOptions(trimmed),
			);
			void navigate({
				to: ".",
				search: (prev) => ({ ...prev, actorId: trimmed }),
			});
		} catch {
			void navigate({
				to: ".",
				search: (prev) => ({ ...prev, actorKey: trimmed }),
			});
		} finally {
			setValue("");
			setOpen(false);
			setIsPending(false);
		}
	};

	const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") void handleSubmit();
		if (e.key === "Escape") {
			setValue("");
			setOpen(false);
		}
	};

	if (!open) {
		return (
			<Button
				size="sm"
				variant="ghost"
				onClick={() => setOpen(true)}
				startIcon={<Icon icon={faMagnifyingGlass} />}
				{...props}
			>
				{copy.goToActor}
			</Button>
		);
	}

	return (
		<Input
			autoFocus
			value={value}
			placeholder="Actor ID or key..."
			className="h-7 text-xs px-2 py-0.5"
			disabled={isPending}
			onChange={(e) => setValue(e.target.value)}
			onKeyDown={handleKeyDown}
			onBlur={() => void handleSubmit()}
		/>
	);
}
