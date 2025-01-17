import * as ActorRpcCallForm from "@/domains/project/forms/actor-rpc-call-form";
import {
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	JsonCode,
	Label,
} from "@rivet-gg/components";

interface ActorRpcProps {
	rpc: string;
}

export function ActorRpc({ rpc }: ActorRpcProps) {
	return (
		<AccordionItem value={rpc}>
			<AccordionTrigger>{rpc}</AccordionTrigger>
			<AccordionContent>
				<ActorRpcCallForm.Form
					onSubmit={(values) => {
						console.log("RPC called", values);
					}}
					defaultValues={{ arguments: [] }}
				>
					<ActorRpcCallForm.Arguments />

					<div className="grid grid-cols-2 gap-4 my-4">
						<ActorRpcCallForm.ExampleCall rpc={rpc} />

						<div className="border p-4 rounded-md relative">
							<Label className="inline-block absolute top-0 -translate-y-1/2 bg-card px-0.5 font-semibold">
								Response
							</Label>
							<JsonCode
								value="// RPC not called"
								editable={false}
								readOnly
							/>
						</div>
					</div>
				</ActorRpcCallForm.Form>
			</AccordionContent>
		</AccordionItem>
	);
}
