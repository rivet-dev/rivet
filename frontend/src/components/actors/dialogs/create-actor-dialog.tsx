import { Rivet } from "@rivetkit/engine-api-full";
import { useMutation } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
import type { DialogContentProps } from "@/components/hooks";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { getRandomKey } from "@/lib/words";
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

interface ContentProps extends DialogContentProps {}

export default function CreateActorDialog({ onClose }: ContentProps) {
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
					datacenter:
						__APP_TYPE__ === "inspector" ? "" : values.datacenter,
					crashPolicy:
						values.crashPolicy || Rivet.CrashPolicy.Restart,
					runnerNameSelector: values.runnerNameSelector || "default",
				});
				onClose?.();
			}}
			defaultValues={{
				name,
				key: getRandomKey(),
				crashPolicy: Rivet.CrashPolicy.Restart,
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
				{["engine", "cloud"].includes(__APP_TYPE__) ? (
					<>
						<ActorCreateForm.PrefillActorName />
						<ActorCreateForm.PrefillRunnerName />
						<ActorCreateForm.PrefillDatacenter />
					</>
				) : null}

				<Accordion type="single" collapsible>
					<AccordionItem value="item-1">
						<AccordionTrigger>Advanced</AccordionTrigger>
						<AccordionContent className="flex gap-4 flex-col">
							{["engine", "cloud"].includes(__APP_TYPE__) ? (
								<>
									<ActorCreateForm.Datacenter />
									<ActorCreateForm.RunnerNameSelector />
									<ActorCreateForm.CrashPolicy />
								</>
							) : null}
							<ActorCreateForm.JsonInput />
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</Flex>
			<DialogFooter>
				<ActorCreateForm.Submit allowPristine type="submit">
					Create
				</ActorCreateForm.Submit>
			</DialogFooter>
		</ActorCreateForm.Form>
	);
}
