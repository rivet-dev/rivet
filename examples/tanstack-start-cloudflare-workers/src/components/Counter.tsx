"use client";

import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "@/actors";

export const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

export function Counter() {
	const [counterId, setCounterId] = useState("default");
	const [count, setCount] = useState<number>(0);

	const counter = useActor({
		name: "counter",
		key: [counterId],
	});

	// Use connStatus from the hook instead of tracking connection state manually
	const isConnected = counter.connStatus === "connected";

	useEffect(() => {
		if (counter.connection && isConnected) {
			counter.connection.getCount().then(setCount);
		}
	}, [counter.connection, isConnected]);

	counter.useEvent("newCount", (newCount: number) => {
		setCount(newCount);
	});

	const increment = async (amount: number) => {
		if (counter.connection) {
			await counter.connection.increment(amount);
		}
	};

	return (
		<div className="w-full max-w-md mx-auto p-4">
			<div className="bg-gray-900 rounded-2xl border border-gray-800 overflow-hidden shadow-xl">
				<div className="p-6 border-b border-gray-800 flex justify-between items-center">
					<h1 className="text-2xl font-semibold text-white">
						Counter Demo
					</h1>
					<div
						className={`flex items-center gap-2 text-sm font-medium ${isConnected ? "text-green-400" : "text-red-400"}`}
					>
						<div
							className={`w-2 h-2 rounded-full ${isConnected ? "bg-green-400" : "bg-red-400"}`}
						></div>
						<span>
							{isConnected ? "Connected" : "Disconnected"}
						</span>
					</div>
				</div>

				<div className="p-6 border-b border-gray-800">
					<div>
						<label
							htmlFor="counterId"
							className="block text-sm font-semibold text-gray-400 mb-2"
						>
							Counter ID
						</label>
						<input
							id="counterId"
							type="text"
							value={counterId}
							onChange={(e) => setCounterId(e.target.value)}
							placeholder="Enter counter ID"
							className="w-full py-2.5 px-3.5 border border-gray-700 rounded-lg text-sm bg-gray-800 text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 transition-all"
						/>
					</div>
				</div>

				<div className="py-12 px-6 text-center bg-black">
					<div className="text-7xl font-bold text-blue-500 leading-none mb-3 tabular-nums">
						{count}
					</div>
					<p className="text-base text-gray-400 font-medium">
						Current Count
					</p>
				</div>

				<div className="p-6 flex gap-3 border-b border-gray-800">
					<button
						onClick={() => increment(1)}
						disabled={!isConnected}
						className="flex-1 py-4 px-4 rounded-xl text-lg font-semibold text-white bg-blue-600 hover:bg-blue-700 hover:-translate-y-0.5 disabled:bg-gray-700 disabled:text-gray-500 disabled:cursor-not-allowed disabled:transform-none transition-all"
					>
						+1
					</button>
					<button
						onClick={() => increment(5)}
						disabled={!isConnected}
						className="flex-1 py-4 px-4 rounded-xl text-lg font-semibold text-white bg-blue-600 hover:bg-blue-700 hover:-translate-y-0.5 disabled:bg-gray-700 disabled:text-gray-500 disabled:cursor-not-allowed disabled:transform-none transition-all"
					>
						+5
					</button>
					<button
						onClick={() => increment(10)}
						disabled={!isConnected}
						className="flex-1 py-4 px-4 rounded-xl text-lg font-semibold text-white bg-blue-600 hover:bg-blue-700 hover:-translate-y-0.5 disabled:bg-gray-700 disabled:text-gray-500 disabled:cursor-not-allowed disabled:transform-none transition-all"
					>
						+10
					</button>
				</div>

				<div className="p-6 bg-gray-800">
					<p className="text-sm text-gray-400 leading-relaxed mb-2">
						This counter is shared across all clients using the same
						Counter ID.
					</p>
					<p className="text-sm text-gray-400 leading-relaxed">
						Try opening this page in multiple tabs or browsers!
					</p>
				</div>
			</div>
		</div>
	);
}
