<script lang="ts">
	import { useActor } from "./rivet.ts";

	let counterName = $state("my-counter");
	let count = $state(0);

	// Connect to the counter actor. The getter re-runs when counterName changes,
	// automatically reconnecting to the new actor instance.
	const counter = useActor(() => ({
		name: "counter",
		key: [counterName],
	}));

	// Subscribe to events broadcast by the actor. Must be called during
	// component initialization alongside useActor.
	counter.onEvent("newCount", (x: number) => {
		count = x;
	});
</script>

<div class="container">
	<div class="card">
		<div class="card-header">
			<h1>Svelte Counter</h1>
		</div>

		<div class="count-display">{count}</div>

		<div class="controls">
			<div class="input-group">
				<label for="counter-name">Counter name</label>
				<input
					id="counter-name"
					type="text"
					bind:value={counterName}
					placeholder="Counter name"
				/>
			</div>

			<button
				onclick={() => counter.increment(1)}
				disabled={!counter.isConnected}
			>
				Increment
			</button>
		</div>

		<p class="status">
			Status: <span class="status-value">{counter.connStatus}</span>
		</p>
	</div>
</div>

<style>
	:global(*) {
		box-sizing: border-box;
		margin: 0;
		padding: 0;
	}

	:global(body) {
		background: #000;
		color: #fff;
		font-family:
			-apple-system,
			BlinkMacSystemFont,
			"Segoe UI",
			"Inter",
			Roboto,
			sans-serif;
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.container {
		width: 100%;
		max-width: 480px;
		padding: 24px;
	}

	.card {
		background: #1c1c1e;
		border: 1px solid #2c2c2e;
		border-radius: 8px;
		overflow: hidden;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
	}

	.card-header {
		background: #2c2c2e;
		padding: 16px 20px;
		border-bottom: 1px solid #2c2c2e;
	}

	.card-header h1 {
		font-size: 18px;
		font-weight: 600;
	}

	.count-display {
		font-size: 72px;
		font-weight: 700;
		text-align: center;
		padding: 32px 20px;
		color: #ff4f00;
	}

	.controls {
		padding: 0 20px 20px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.input-group label {
		font-size: 12px;
		color: #8e8e93;
		font-weight: 500;
	}

	input {
		background: #2c2c2e;
		border: 1px solid #3a3a3c;
		border-radius: 8px;
		color: #fff;
		font-size: 14px;
		padding: 12px 16px;
		width: 100%;
		transition: border-color 200ms ease;
	}

	input:focus {
		outline: none;
		border-color: #ff4f00;
		box-shadow: 0 0 0 3px rgba(255, 79, 0, 0.2);
	}

	input::placeholder {
		color: #6e6e73;
	}

	button {
		background: #ff4f00;
		border: none;
		border-radius: 8px;
		color: #fff;
		cursor: pointer;
		font-size: 14px;
		font-weight: 600;
		padding: 12px 20px;
		width: 100%;
		transition: opacity 200ms ease;
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.status {
		font-size: 12px;
		color: #8e8e93;
		padding: 12px 20px;
		border-top: 1px solid #2c2c2e;
	}

	.status-value {
		color: #30d158;
		font-weight: 500;
	}
</style>
