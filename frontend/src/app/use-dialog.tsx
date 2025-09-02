import { createDialogHook, useDialog } from "@/components/actors";

const d = useDialog as typeof useDialog &
	Record<string, ReturnType<typeof createDialogHook>>;
d.CreateNamespace = createDialogHook(
	import("@/app/dialogs/create-namespace-dialog"),
);

export { d as useDialog };
