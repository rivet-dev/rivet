import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	H2,
	Link,
	Strong,
	Ul,
} from "@rivet-gg/components";

export function FaqSection() {
	return (
		<>
			<H2 mt="10">Frequently Asked Questions</H2>
			<Accordion type="single" collapsible>
				<AccordionItem value="item-1">
					<AccordionTrigger>What is Rivet?</AccordionTrigger>
					<AccordionContent>
						<p>
							Rivet is a multiplayert tooling that provides a
							suite of tools to help you build and deploy
							multiplayer projects. Rivet provides a scalable,
							secure, and reliable infrastructure for:
						</p>
						<Ul>
							<li>
								Dynamic Servers for auto-scaling project lobbies{" "}
							</li>
							<li>
								DDoS mitigation and managed WebSocket SSL &
								TCP+TLS termination
							</li>
							<li>Streamlined DevOps for teams</li>
							<li>Unified logging & monitoring & analytics</li>
							<li>No downtime deploys with easy rollbacks</li>
						</Ul>
					</AccordionContent>
				</AccordionItem>
				<AccordionItem value="item-2">
					<AccordionTrigger>
						What Project Engines are supported?
					</AccordionTrigger>
					<AccordionContent>
						<p>
							Rivet supports all major project engines including
							Unity, Unreal Engine, Godot. It also provides SDKs
							for popular languages like C#, C++, and JavaScript,
							so you can use Rivet with any project engine even if
							it's your own custom engine.
						</p>
					</AccordionContent>
				</AccordionItem>
				<AccordionItem value="item-3">
					<AccordionTrigger>Is Rivet open-source?</AccordionTrigger>
					<AccordionContent>
						<p>
							<Strong>Yes, Rivet is open-source.</Strong>You can
							find our repositories on{" "}
							<Link
								href="https://rivet.gg"
								target="_blank"
								rel="noreferrer"
							>
								GitHub
							</Link>
							. We welcome contributions, bug reports, and feature
							requests from the community.
						</p>
					</AccordionContent>
				</AccordionItem>
				<AccordionItem value="item-4">
					<AccordionTrigger>Is Rivet free?</AccordionTrigger>
					<AccordionContent>
						<p>
							<Strong>Yes, Rivet is free.</Strong> You can use
							Rivet for free with no limits on the number of
							players. We offer premium support plans for teams
							that require additional support. For more
							information,
							<Link
								href="https://rivet.gg/pricing"
								target="_blank"
								rel="noreferrer"
							>
								please visit our pricing page
							</Link>
							.
						</p>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</>
	);
}
