import { createRivetKit } from "@rivetkit/react";
import { useMemo, useState } from "react";
import {
	Badge,
	Button,
	ButtonGroup,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Footer,
	FormField,
	GridDecoration,
	Navigation,
	RenderLogo,
	ThemeToggle,
} from "render-dds";
import { rivetClientBase, type AppRegistry } from "./rivet-client";

export function App() {
	const rivet = useMemo(() => createRivetKit<AppRegistry>(rivetClientBase()), []);
	const { useActor } = rivet;

	const [actorKey, setActorKey] = useState("default");
	const [total, setTotal] = useState(0);

	const counter = useActor({ name: "counter", key: [actorKey] });
	const live = Boolean(counter.connection);

	counter.useEvent("newCount", (next: number) => setTotal(next));

	const add = async (delta: number) => {
		if (counter.connection) await counter.connection.increment(delta);
	};

	return (
		<div className="relative flex min-h-screen flex-col bg-background text-foreground">
			<GridDecoration position="top-right" className="pointer-events-none" height={280} opacity={0.28} width={280} />
			<GridDecoration position="bottom-left" className="pointer-events-none" height={220} opacity={0.2} width={220} />

			<Navigation
				className="relative z-10 border-b border-border bg-background/80 backdrop-blur-sm"
				logo={
					<div className="flex items-center gap-3">
						<RenderLogo variant="mark" height={28} />
						<div className="flex flex-col">
							<span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">RivetKit</span>
							<span className="text-sm font-semibold leading-tight text-foreground">React Integration</span>
						</div>
					</div>
				}
				actions={
					<div className="flex items-center gap-2">
						<Badge variant={live ? "green" : "red-light"}>
							<span className="inline-flex items-center gap-1.5">
								<span className={live ? "size-1.5 rounded-full bg-green-500" : "size-1.5 animate-pulse rounded-full bg-red-400"} />
								{live ? "Connected" : "Connecting"}
							</span>
						</Badge>
						<ThemeToggle size="sm" variant="outline" />
					</div>
				}
			/>

			<main className="relative z-10 flex min-h-0 flex-1 flex-col items-center justify-center px-4 py-12 sm:px-6">
				<h1 className="max-w-3xl text-center font-sans text-4xl font-bold tracking-tight text-foreground sm:text-5xl">
					React Integration
				</h1>
				<p className="mx-auto mt-4 max-w-xl text-center text-base text-muted-foreground sm:text-lg">
					Type-safe React hooks for Rivet Actors — connect your UI to real-time backend state with <code className="rounded bg-muted px-1.5 py-0.5 text-sm">useActor</code>.
				</p>

				<div className="mt-10 w-full max-w-md">
					<Card variant="elevated" className="overflow-hidden border-border shadow-lg shadow-black/5 dark:shadow-black/20">
						<CardContent className="border-b border-border px-5 py-5">
							<FormField
								id="actor-key"
								label="Actor key"
								helperText="Same key in another window = same running total."
								value={actorKey}
								onChange={(e) => setActorKey(e.target.value)}
								placeholder="e.g. default"
								autoComplete="off"
							/>
						</CardContent>

						<div className="border-b border-border bg-muted/35 px-6 py-10 text-center dark:bg-zinc-950/50">
							<p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">Current value</p>
							<p className="mt-3 text-5xl font-bold tabular-nums tracking-tight text-primary sm:text-6xl" aria-live="polite">
								{total}
							</p>
						</div>

						<div className="px-5 py-5">
							<ButtonGroup className="grid w-full grid-cols-3 gap-2" orientation="horizontal">
								{([1, 5, 10] as const).map((n) => (
									<Button key={n} variant="default" className="min-h-11 flex-1 font-semibold" disabled={!live} onClick={() => add(n)}>
										+{n}
									</Button>
								))}
							</ButtonGroup>
						</div>
					</Card>
				</div>
			</main>

			<section className="flex justify-center px-4 pb-10 pt-2 md:pb-14">
				<div className="w-full max-w-md">
					<Card variant="outlined" className="border-dashed border-border/80 text-center">
						<CardHeader className="pb-2">
							<CardTitle className="text-base">Deploy on Render</CardTitle>
						</CardHeader>
						<CardContent className="flex justify-center pt-0">
							<a
								href="https://render.com/deploy?repo=https://github.com/ojusave/rivet-react"
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex shrink-0"
								aria-label="Deploy to Render"
							>
								<img
									src="https://render.com/images/deploy-to-render-button.svg"
									alt=""
									width={155}
									height={40}
									decoding="async"
								/>
							</a>
						</CardContent>
					</Card>
				</div>
			</section>

			<Footer
				centered
				className="relative z-10 mt-auto border-t border-border bg-background/90"
				copyright="react-render"
				links={[
					{ label: "Render", href: "https://render.com" },
					{ label: "Rivet", href: "https://rivet.dev" },
				]}
			/>
		</div>
	);
}
