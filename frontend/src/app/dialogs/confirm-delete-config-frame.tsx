import { useMutation } from "@tanstack/react-query";
import { Button, type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";

interface ConfirmDeleteConfigContentProps extends DialogContentProps {
	name: string;
}

export default function ConfirmDeleteConfigContent({
	onClose,
	name,
}: ConfirmDeleteConfigContentProps) {
	const provider = useEngineCompatDataProvider();
	const { mutate, isPending } = useMutation(
		provider.deleteRunnerConfigMutationOptions({
			onSuccess: async () => {
				await queryClient.invalidateQueries(
					provider.runnerConfigsQueryOptions(),
				);
				onClose?.();
			},
		}),
	);

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>Confirm deletion of '{name}' provider</div>
				</Frame.Title>
				<Frame.Description>
					This action cannot be undone. Are you sure you want to
					delete this configuration?
				</Frame.Description>
			</Frame.Header>
			<Frame.Footer>
				<Button
					variant="destructive"
					isLoading={isPending}
					onClick={() => {
						mutate(name);
					}}
				>
					Delete
				</Button>
				<Button variant="secondary" onClick={onClose}>
					Cancel
				</Button>
			</Frame.Footer>
		</>
	);
}
