import { useMutation } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
import type { DialogContentProps } from "@/components/hooks";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../../ui/dialog";
import { Flex } from "../../ui/flex";
import { useActorsView } from "../actors-view-context-provider";
import { useDataProvider } from "../data-provider";
import * as ActorCreateForm from "../form/actor-create-form";
import { CrashPolicy } from "../queries";

interface ContentProps extends DialogContentProps {
	namespace: string;
}

export default function CreateActorDialog({
	onClose,
	namespace,
}: ContentProps) {
	const { mutateAsync } = useMutation(
		useDataProvider().createActorMutationOptions(),
	);
	const name = useSearch({
		from: "/_context",
		select: (state) => state.n?.[0],
	});

	const { copy } = useActorsView();

	return (
		<ActorCreateForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					name: values.name,
					input: values.input ? JSON.parse(values.input) : undefined,
					key: values.key,
					crashPolicy: values.crashPolicy || CrashPolicy.Destroy,
					runnerNameSelector: values.runnerNameSelector || "default",
				});
				onClose?.();
			}}
			defaultValues={{
				name,
				crashPolicy: CrashPolicy.Destroy,
				region: "auto",
			}}
		>
			<DialogHeader>
				<DialogTitle>{copy.createActorModal.title(name)}</DialogTitle>
				<DialogDescription>
					{copy.createActorModal.description}
				</DialogDescription>
			</DialogHeader>
			<Flex gap="4" direction="col">
				{!name ? <ActorCreateForm.Build /> : null}
				<ActorCreateForm.Keys />
				<ActorCreateForm.ActorPreview />
				{__APP_TYPE__ === "engine" ? (
					<ActorCreateForm.PrefillRunnerName namespace={namespace} />
				) : null}

				<Accordion type="single" collapsible>
					<AccordionItem value="item-1">
						<AccordionTrigger>Advanced</AccordionTrigger>
						<AccordionContent className="flex gap-4 flex-col">
							{__APP_TYPE__ === "engine" ? (
								<>
									<ActorCreateForm.Region />
									<ActorCreateForm.RunnerNameSelector
										namespace={namespace}
									/>
									<ActorCreateForm.CrashPolicy />
								</>
							) : null}
							<ActorCreateForm.JsonInput />
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</Flex>
			<DialogFooter>
				<ActorCreateForm.Submit type="submit">
					Create
				</ActorCreateForm.Submit>
			</DialogFooter>
		</ActorCreateForm.Form>
	);
}
