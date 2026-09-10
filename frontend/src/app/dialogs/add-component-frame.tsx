import { useNavigate } from "@tanstack/react-router";
import { CONNECT_DURABLE_STREAMS_MODAL } from "@/app/dialogs/connect-provider-sheet";
import { Frame } from "@/components";
import {
	getProductDocsUrl,
	ProductPicker,
} from "@/components/products/product-picker";

export default function AddComponentFrameContent({
	onClose,
}: {
	onClose?: () => void;
}) {
	const navigate = useNavigate();
	return (
		<>
			<Frame.Header>
				<Frame.Title>Add a component</Frame.Title>
				<Frame.Description>
					Pick what you want to add to this project.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<ProductPicker
					ariaLabel="Add a component"
					onSelect={(target) => {
						// Products are added by writing code, so hand off to
						// the docs. Services are connected in the dashboard,
						// so open their setup sheet instead.
						if (target === "durable-streams") {
							void navigate({
								to: ".",
								search: (s) => ({
									...(s as Record<string, unknown>),
									modal: CONNECT_DURABLE_STREAMS_MODAL,
								}),
							});
						} else {
							window.open(
								getProductDocsUrl(target),
								"_blank",
								"noopener,noreferrer",
							);
						}
						onClose?.();
					}}
				/>
			</Frame.Content>
		</>
	);
}
