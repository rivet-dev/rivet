import { CardGroup, Card } from "@/components/Card";
import { deployOptions } from "@/data/deploy/shared";

export function Hosting() {
	const hostingProviders = deployOptions;

	return (
		<>
			<p>
				By default, Rivet stores actor state on the local file system.
			</p>

			<p>
				To scale Rivet in production, follow a guide to deploy to your
				hosting provider of choice:
			</p>

			<CardGroup>
				{hostingProviders
					.filter((x) => !x.specializedPlatform)
					.map(({ displayName: title, href, icon }) => (
						<Card
							key={href}
							title={title}
							href={href}
							icon={icon}
						/>
					))}
			</CardGroup>
		</>
	);
}
