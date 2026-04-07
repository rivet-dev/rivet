import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useState } from "react";
import {
	Alert,
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
	const rivet = useMemo(
		() => createRivetKit<AppRegistry>(rivetClientBase()),
		[],
	);
	const { useActor } = rivet;

	const [actorKey, setActorKey] = useState("default");
	const [total, setTotal] = useState(0);

	const counter = useActor({
		name: "counter",
		key: [actorKey],
	});

	useEffect(() => {
		if (counter.connection) {
			counter.connection.getCount().then(setTotal);
		}
	}, [counter.connection]);

	counter.useEvent("newCount", (next: number) => {
		setTotal(next);
	});

	const add = async (delta: number) => {
		if (counter.connection) {
			await counter.connection.increment(delta);
		}
	};

	const live = Boolean(counter.connection);

	return (
		<div className="relative flex min-h-screen flex-col bg-background text-foreground">
			<GridDecoration
				position="top-right"
				className="pointer-events-none"
				height={280}
				opacity={0.28}
				width={280}
			/>
			<GridDecoration
				position="bottom-left"
				className="pointer-events-none"
				height={220}
				opacity={0.2}
				width={220}
			/>

			<Navigation
				className="relative z-10 border-b border-border bg-background/80 backdrop-blur-sm"
				logo={
					<div className="flex items-center gap-3">
						<RenderLogo variant="mark" height={28} />
						<div className="flex flex-col gap-0">
							<span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
								RivetKit
							</span>
							<span className="text-sm font-semibold leading-tight text-foreground">
								Render sample
							</span>
						</div>
					</div>
				}
				actions={
					<div className="flex items-center gap-2 sm:gap-3">
						<Badge variant={live ? "green" : "red-light"}>
							<span className="inline-flex items-center gap-1.5">
								<span
									className={
										live
											? "size-1.5 rounded-full bg-green-500"
											: "size-1.5 animate-pulse rounded-full bg-red-400"
									}
									aria-hidden
								/>
								{live ? "Connected" : "Connecting"}
							</span>
						</Badge>
						<ThemeToggle size="sm" variant="outline" />
					</div>
				}
			/>

			<main className="relative z-10 flex min-h-0 flex-1 flex-col">
				<section
					aria-labelledby="hero-heading"
					className="flex flex-1 flex-col justify-center px-4 py-12 text-center sm:px-6 md:py-16 lg:py-20"
				>
					<div className="mx-auto flex w-full max-w-4xl flex-col items-center">
						<h1
							id="hero-heading"
							className="max-w-3xl font-sans text-4xl font-bold tracking-tight text-foreground sm:text-5xl md:text-6xl"
						>
							Shared counter
						</h1>
						<p className="mx-auto mt-4 max-w-2xl text-pretty text-base leading-relaxed text-muted-foreground sm:text-lg md:text-xl">
							One actor, any number of tabs. Use the same actor key everywhere to share state —
							powered by RivetKit on Render or Rivet Cloud.
						</p>

						<div className="mt-10 w-full max-w-md md:mt-12">
							<Card
								variant="elevated"
								className="overflow-hidden border-border text-left shadow-lg shadow-black/5 dark:shadow-black/20"
							>
								<div className="flex flex-col items-center justify-between gap-1 border-b border-border bg-muted/30 px-5 py-4 text-center sm:flex-row sm:text-left dark:bg-muted/15">
									<span className="text-sm font-semibold text-foreground">Your counter</span>
									<span className="text-xs text-muted-foreground">Actor: counter</span>
								</div>

								<CardContent className="space-y-0 border-b border-border px-5 py-5">
									<FormField
										id="actor-key"
										helperText="Same key in another window = same running total."
										label="Actor key"
										value={actorKey}
										onChange={(e) => setActorKey(e.target.value)}
										placeholder="e.g. default"
										autoComplete="off"
									/>
								</CardContent>

								<div className="border-b border-border bg-muted/35 px-6 py-10 text-center dark:bg-zinc-950/50 md:py-12">
									<p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
										Current value
									</p>
									<p
										className="mt-3 text-5xl font-bold tabular-nums tracking-tight text-primary sm:text-6xl md:text-7xl"
										aria-live="polite"
									>
										{total}
									</p>
								</div>

								<div className="px-5 py-5">
									<ButtonGroup
										className="grid w-full grid-cols-3 gap-2 sm:flex sm:flex-row sm:justify-center"
										orientation="horizontal"
									>
										{([1, 5, 10] as const).map((n) => (
											<Button
												key={n}
												type="button"
												className="min-h-11 flex-1 font-semibold"
												variant="default"
												disabled={!live}
												onClick={() => add(n)}
											>
												+{n}
											</Button>
										))}
									</ButtonGroup>
								</div>

								<div className="border-t border-border bg-muted/20 px-0 py-0 dark:bg-muted/10">
									<Alert
										className="border-0 bg-transparent text-left"
										showIcon
										title="Try it"
										variant="help"
									>
										<p>Open this page in two tabs with the same actor key — both stay in sync.</p>
									</Alert>
								</div>
							</Card>
						</div>
					</div>
				</section>

				<section className="flex justify-center px-4 pb-10 pt-2 md:pb-14">
					<div className="w-full max-w-md">
						<Card variant="outlined" className="border-dashed border-border/80 text-center">
							<CardHeader className="pb-2">
								<CardTitle className="text-base">Deploy on Render</CardTitle>
							</CardHeader>
							<CardContent className="flex justify-center pt-0">
								<a
									href="https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-render"
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
			</main>

			<Footer
				centered
				className="relative z-10 mt-auto border-t border-border bg-background/90"
				copyright="RivetKit hello-world-render"
				links={[
					{ label: "Render", href: "https://render.com" },
					{ label: "Rivet", href: "https://rivet.dev" },
				]}
			/>
		</div>
	);
}
