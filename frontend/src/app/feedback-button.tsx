import { faMessage, Icon } from "@rivet-gg/icons";
import { useEffect, useState } from "react";
import {
	Button,
	cn,
	Kbd,
	Popover,
	PopoverContent,
	PopoverTrigger,
	toast,
} from "@/components";
import { FEEDBACK_FORM_ID } from "@/components/lib/constants";
import { posthog } from "@/lib/posthog";

export function FeedbackButton({ source = "web" }: { source?: string }) {
	const [open, setOpen] = useState(false);
	const [value, setValue] = useState("");
	const [submitting, setSubmitting] = useState(false);

	// `F` keyboard shortcut to open the popover. Ignore when the user is
	// already typing in an input/textarea/contenteditable somewhere else.
	useEffect(() => {
		const handler = (e: KeyboardEvent) => {
			if (e.key.toLowerCase() !== "f") return;
			if (e.metaKey || e.ctrlKey || e.altKey) return;
			const target = e.target as HTMLElement | null;
			if (!target) return;
			const tag = target.tagName;
			if (
				tag === "INPUT" ||
				tag === "TEXTAREA" ||
				target.isContentEditable
			) {
				return;
			}
			e.preventDefault();
			setOpen(true);
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, []);

	const submit = async () => {
		const trimmed = value.trim();
		if (!trimmed || submitting) return;
		setSubmitting(true);
		try {
			posthog.capture("survey sent", {
				utm_source: source,
				$survey_id: FEEDBACK_FORM_ID,
				$survey_response: `feedback from ${source}: ${trimmed}`,
			});
			setValue("");
			setOpen(false);
			toast.success("Thanks — feedback sent.");
		} finally {
			setSubmitting(false);
		}
	};

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className={cn(
						"gap-2 text-muted-foreground hover:text-foreground",
						open && "bg-foreground/[0.06] text-foreground",
					)}
					startIcon={<Icon icon={faMessage} className="size-3.5" />}
				>
					Feedback
				</Button>
			</PopoverTrigger>
			<PopoverContent
				align="end"
				className="w-[28rem] p-3"
				onOpenAutoFocus={(e) => {
					// Let the textarea autofocus instead of the container.
					e.preventDefault();
				}}
			>
				<textarea
					value={value}
					onChange={(e) => setValue(e.target.value)}
					autoFocus
					rows={4}
					placeholder="Have an idea to improve the product? Tell the Rivet team..."
					className={cn(
						"w-full resize-none rounded-md border border-foreground/10 bg-foreground/[0.02] p-3 text-sm",
						"placeholder:text-muted-foreground/60 focus-visible:outline-none focus-visible:border-foreground/30",
					)}
					onKeyDown={(e) => {
						if (
							(e.metaKey || e.ctrlKey) &&
							e.key === "Enter"
						) {
							e.preventDefault();
							submit();
						}
					}}
				/>
				<div className="mt-3 flex items-center justify-between gap-3">
					<p className="text-xs text-muted-foreground">
						Need help?{" "}
						<a
							href="https://rivet.dev/discord"
							target="_blank"
							rel="noopener noreferrer"
							className="font-medium text-foreground hover:underline"
						>
							Join Discord
						</a>{" "}
						or{" "}
						<a
							href="https://rivet.dev/docs"
							target="_blank"
							rel="noopener noreferrer"
							className="font-medium text-foreground hover:underline"
						>
							see docs
						</a>
						.
					</p>
					<Button
						type="button"
						size="sm"
						variant="secondary"
						isLoading={submitting}
						disabled={value.trim().length === 0}
						onClick={submit}
						endIcon={
							<span className="flex items-center gap-0.5">
								<Kbd>⌘</Kbd>
								<Kbd>↵</Kbd>
							</span>
						}
					>
						Send
					</Button>
				</div>
			</PopoverContent>
		</Popover>
	);
}
