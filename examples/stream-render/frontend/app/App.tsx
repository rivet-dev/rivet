import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useState } from "react";
import {
	Badge,
	Button,
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

	const [topValues, setTopValues] = useState<number[]>([]);
	const [newValue, setNewValue] = useState<number>(0);
	const [totalCount, setTotalCount] = useState(0);
	const [highestValue, setHighestValue] = useState<number | null>(null);

	const stream = useActor({ name: "streamProcessor", key: ["global"] });
	const live = Boolean(stream.connection);

	useEffect(() => {
		if (stream.connection) {
			stream.connection.getStats().then((s) => {
				setTopValues(s.topValues);
				setTotalCount(s.totalCount);
				setHighestValue(s.highestValue);
			});
		}
	}, [stream.connection]);

	stream.useEvent("updated", (u: { topValues: number[]; totalCount: number; highestValue: number | null }) => {
		setTopValues(u.topValues);
		setTotalCount(u.totalCount);
		setHighestValue(u.highestValue);
	});

	const addValue = async () => {
		if (stream.connection && !isNaN(newValue)) {
			await stream.connection.addValue(newValue);
			setNewValue(0);
		}
	};

	const reset = async () => {
		if (stream.connection) {
			const r = await stream.connection.reset();
			setTopValues(r.topValues);
			setTotalCount(r.totalCount);
			setHighestValue(r.highestValue);
		}
	};

	return (
		<div className="relative flex min-h-screen flex-col bg-background text-foreground">
			<GridDecoration position="top-right" className="pointer-events-none" height={280} opacity={0.28} width={280} />

			<Navigation
				className="relative z-10 border-b border-border bg-background/80 backdrop-blur-sm"
				logo={
					<div className="flex items-center gap-3">
						<RenderLogo variant="mark" height={28} />
						<div className="flex flex-col">
							<span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">RivetKit</span>
							<span className="text-sm font-semibold leading-tight text-foreground">Stream Processor</span>
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

			<main className="relative z-10 mx-auto flex w-full max-w-3xl flex-1 flex-col gap-6 px-4 py-10 sm:px-6">
				<div className="text-center">
					<h1 className="font-sans text-4xl font-bold tracking-tight text-foreground sm:text-5xl">
						Stream Processor
					</h1>
					<p className="mx-auto mt-3 max-w-xl text-base text-muted-foreground sm:text-lg">
						Real-time top-K processing — add values and watch the leaderboard update live across all clients.
					</p>
				</div>

				<div className="grid gap-3 sm:grid-cols-3">
					{[
						{ label: "Total Values", value: totalCount },
						{ label: "Highest", value: highestValue?.toLocaleString() ?? "\u2014" },
						{ label: "Top Count", value: topValues.length },
					].map((s) => (
						<Card key={s.label} variant="outlined" className="border-border text-center">
							<CardContent className="py-4">
								<p className="text-2xl font-bold tabular-nums text-primary">{s.value}</p>
								<p className="mt-1 text-xs text-muted-foreground">{s.label}</p>
							</CardContent>
						</Card>
					))}
				</div>

				<div className="grid gap-6 md:grid-cols-2">
					<Card variant="elevated" className="border-border shadow-lg shadow-black/5 dark:shadow-black/20">
						<div className="border-b border-border bg-muted/30 px-5 py-4 dark:bg-muted/15">
							<span className="text-sm font-semibold text-foreground">Top 3 Values</span>
						</div>
						<CardContent className="px-5 py-4">
							{topValues.length === 0 ? (
								<p className="py-8 text-center text-sm italic text-muted-foreground">No values yet.</p>
							) : (
								<div className="space-y-2">
									{topValues.map((v, i) => (
										<div key={`${v}-${i}`} className="flex items-center justify-between rounded-md border border-border bg-muted/20 px-4 py-3 dark:bg-muted/10">
											<span className="text-sm font-medium text-muted-foreground">#{i + 1}</span>
											<span className="text-lg font-bold tabular-nums text-foreground">{v.toLocaleString()}</span>
										</div>
									))}
								</div>
							)}
						</CardContent>
					</Card>

					<Card variant="elevated" className="border-border shadow-lg shadow-black/5 dark:shadow-black/20">
						<div className="border-b border-border bg-muted/30 px-5 py-4 dark:bg-muted/15">
							<span className="text-sm font-semibold text-foreground">Add Value</span>
						</div>
						<CardContent className="space-y-4 px-5 py-5">
							<form
								onSubmit={(e) => {
									e.preventDefault();
									addValue();
								}}
							>
								<FormField
									id="value"
									label="Number"
									type="number"
									value={newValue || ""}
									onChange={(e) => setNewValue(Number(e.target.value))}
									placeholder="Enter any number"
									disabled={!live}
								/>
								<Button variant="default" className="mt-3 w-full" type="submit" disabled={!live || isNaN(newValue)}>
									Add to Stream
								</Button>
							</form>
							<div className="flex gap-2">
								<Button
									variant="outline"
									className="flex-1"
									onClick={() => setNewValue(Math.floor(Math.random() * 1000) + 1)}
								>
									Random
								</Button>
								<Button variant="destructive" className="flex-1" disabled={!live} onClick={reset}>
									Reset
								</Button>
							</div>
						</CardContent>
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
								href="https://render.com/deploy?repo=https://github.com/ojusave/rivet-stream"
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
				copyright="stream-render"
				links={[
					{ label: "Render", href: "https://render.com" },
					{ label: "Rivet", href: "https://rivet.dev" },
				]}
			/>
		</div>
	);
}
