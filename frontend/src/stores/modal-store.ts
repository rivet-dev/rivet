import { useStore } from "@tanstack/react-store";
import { Store } from "@tanstack/store";

interface ModalState {
	openModal: {
		id: string;
		dialogKey: string;
		props?: Record<string, unknown>;
	} | null;
}

const initialState: ModalState = {
	openModal: null,
};

export const modalStore = new Store(initialState);

let modalIdCounter = 0;

export const modalActions = {
	openModal: (dialogKey: string, props?: Record<string, unknown>) => {
		const id = `modal-${++modalIdCounter}`;
		modalStore.setState((state) => ({
			...state,
			openModal: {
				id,
				dialogKey,
				props,
			},
		}));
		return id;
	},

	closeModal: () => {
		modalStore.setState((state) => ({
			...state,
			openModal: null,
		}));
	},
};

export const useModalStore = () => {
	return useStore(modalStore);
};

export const useOpenModal = () => {
	return useStore(modalStore, (state) => state.openModal);
};