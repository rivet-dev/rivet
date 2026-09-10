import { DurableStreamsConnect } from "@/app/getting-started";
import { type DialogContentProps, Frame } from "@/components";
import { getProduct, ProductMark } from "@/components/products/product-picker";

interface ConnectDurableStreamsFrameContentProps extends DialogContentProps {}

// "Add Durable Streams" sheet. Durable Streams is a service rather than a
// provider, so instead of a runner config form this reuses the onboarding
// connect step: the managed service URL on cloud, the worker container
// command everywhere else.
export default function ConnectDurableStreamsFrameContent(
	_props: ConnectDurableStreamsFrameContentProps,
) {
	const product = getProduct("durable-streams");
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div className="flex items-center gap-2">
						Add
						<ProductMark
							fileName={product.markFileName}
							section={product.section}
						/>
						{product.label}
					</div>
				</Frame.Title>
				<Frame.Description>{product.description}</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<DurableStreamsConnect />
			</Frame.Content>
		</>
	);
}
