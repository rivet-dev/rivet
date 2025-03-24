import * as FeedbackForm from "@/forms/feedback-form";
import type { DialogContentProps } from "@/hooks/use-dialog";
import { FEEDBACK_FORM_ID } from "@/lib/data/constants";
import {
	Button,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	Flex,
	Text,
} from "@rivet-gg/components";

import { usePostHog } from "posthog-js/react";
import { useState } from "react";

interface ContentProps extends DialogContentProps {
	source?: string;
}

export default function FeedbackDialogContent({
	onClose,
	source = "web",
}: ContentProps) {
	const posthog = usePostHog();

	const [isSubmit, setIsSubmit] = useState(false);

	if (isSubmit) {
		return (
			<>
				<DialogHeader>
					<DialogTitle>Feedback sent!</DialogTitle>
				</DialogHeader>
				<Text>
					All submissions are read by humans, we appreciate your
					feedback.
				</Text>
				<DialogFooter>
					<Button variant="secondary" onClick={onClose}>
						Close
					</Button>
				</DialogFooter>
			</>
		);
	}

	return (
		<>
			<FeedbackForm.Form
				onSubmit={async (values) => {
					posthog.capture("survey sent", {
						utm_source: source,
						$survey_id: FEEDBACK_FORM_ID,
						$survey_response: `${values.type} from ${source}: ${values.feedback}`,
					});
					setIsSubmit(true);
				}}
				defaultValues={{ type: "bug", feedback: "" }}
			>
				<DialogHeader>
					<DialogTitle>Feedback</DialogTitle>
				</DialogHeader>
				<Flex gap="4" direction="col">
					<FeedbackForm.Type />
					<FeedbackForm.Feedback />
				</Flex>
				<DialogFooter>
					<FeedbackForm.Submit type="submit">
						Send
					</FeedbackForm.Submit>
				</DialogFooter>
			</FeedbackForm.Form>
		</>
	);
}
