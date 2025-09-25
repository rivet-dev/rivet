"use client";
import { useDialog } from "@/app/use-dialog";
import { modalActions, useOpenModal } from "@/stores/modal-store";

export function ModalRenderer() {
	const openModal = useOpenModal();

	if (!openModal) {
		return null;
	}

	const DialogComponent = getDialogComponent(openModal.dialogKey);
	if (!DialogComponent) {
		console.warn(
			`Dialog component not found for key: ${openModal.dialogKey}`,
		);
		return null;
	}

	return (
		<DialogComponent
			{...(openModal.props || {})}
			dialogProps={{
				open: true,
				onOpenChange: (open: boolean) => {
					if (!open) {
						modalActions.closeModal();
					}
				},
			}}
		/>
	);
}

function getDialogComponent(dialogKey: string) {
	const dialogs = useDialog;
	const dialog = dialogs[dialogKey];

	if (!dialog || typeof dialog !== "function") {
		return null;
	}

	// Access the Dialog component from the hook
	return dialog.Dialog;
}
