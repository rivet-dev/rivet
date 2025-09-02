import type { FormattedCode } from "../worker/actor-worker-schema";

export function ActorConsoleLogFormatted({ tokens }: FormattedCode) {
	return (
		<>
			{tokens.map((tokensLine, index) => (
				<span
					// biome-ignore lint/suspicious/noArrayIndexKey: we're using the index as a key here because the array is static
					key={index}
					className="block"
				>
					{tokensLine.map((token, index) => (
						<span
							// biome-ignore lint/suspicious/noArrayIndexKey: we're using the index as a key here because the array is static
							key={index}
							style={{
								color: token.color,
							}}
						>
							{token.content}
						</span>
					))}
				</span>
			))}
		</>
	);
}
