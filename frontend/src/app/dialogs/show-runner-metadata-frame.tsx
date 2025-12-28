import { useSuspenseQuery } from "@tanstack/react-query";
import { type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

interface ShowRunnerMetadataFrameContentProps extends DialogContentProps {
	runnerId: string;
}

export default function ShowRunnerMetadataFrameContent({
	runnerId,
}: ShowRunnerMetadataFrameContentProps) {
	const provider = useEngineCompatDataProvider();

	const { data: runners } = useSuspenseQuery({
		...provider.runnersQueryOptions(),
	});

	const runner = runners?.find((r) => r.runnerId === runnerId);

	if (!runner) {
		return (
			<>
				<Frame.Header>
					<Frame.Title>Runner Metadata</Frame.Title>
				</Frame.Header>
				<Frame.Content>Runner not found.</Frame.Content>
			</>
		);
	}

	const metadataJson = runner.metadata
		? JSON.stringify(runner.metadata, null, 2)
		: "{}";

	return (
		<>
			<Frame.Header>
				<Frame.Title>Runner Metadata</Frame.Title>
				<Frame.Description>
					Metadata for runner: {runner.name}
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<pre className="bg-muted p-4 rounded-md overflow-auto max-h-[60vh] text-sm">
					<code>{metadataJson}</code>
				</pre>
			</Frame.Content>
		</>
	);
}
