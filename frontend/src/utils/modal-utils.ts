import type { useDialog } from "@/app/use-dialog";
import { modalActions } from "@/stores/modal-store";

export const modal = {
	open: (
		dialogKey: keyof typeof useDialog,
		props?: Record<string, unknown>,
	) => {
		return modalActions.openModal(dialogKey, props);
	},

	close: () => {
		modalActions.closeModal();
	},
};

export type ModalApi = typeof modal;
