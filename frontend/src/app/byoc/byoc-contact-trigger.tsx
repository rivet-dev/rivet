import { type ReactNode, useState } from "react";
import { useDialog } from "@/app/use-dialog";

export function ByocContactTrigger({
	children,
}: {
	children: (open: () => void) => ReactNode;
}) {
	const [isOpen, setOpen] = useState(false);
	const Dialog = useDialog.ByocContact.Dialog;
	return (
		<>
			{children(() => setOpen(true))}
			<Dialog
				dialogProps={{ open: isOpen, onOpenChange: setOpen }}
				dialogContentProps={{
					className: "w-[72rem] max-w-[calc(100vw-2rem)] gap-4",
				}}
			/>
		</>
	);
}
