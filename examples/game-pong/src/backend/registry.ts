import { type ActorContextOf, actor, setup } from "rivetkit";
import {
	BALL_SPEED,
	BALL_SIZE,
	CANVAS_HEIGHT,
	CANVAS_WIDTH,
	PADDLE_HEIGHT,
	PADDLE_SPEED,
	PADDLE_WIDTH,
	TICK_RATE,
	type GameState,
	type MatchResult,
	type PlayerSide,
} from "../shared/constants";

// Matchmaking coordinator - pairs players together
export const matchmaker = actor({
	state: {
		waitingConnId: null as string | null,
	},

	onDisconnect: (c, conn) => {
		if (c.state.waitingConnId === conn.id) {
			c.state.waitingConnId = null;
		}
	},

	actions: {
		findMatch: (c): MatchResult => {
			const waitingConnId = c.state.waitingConnId;

			if (waitingConnId && waitingConnId !== c.conn.id) {
				// Found opponent - create match and notify waiting player
				const matchId = `match-${Date.now()}`;
				c.state.waitingConnId = null;
				c.conns.get(waitingConnId)?.send("matched", { matchId });
				return { matchId, status: "matched" };
			}

			// No opponent - wait in queue
			c.state.waitingConnId = c.conn.id;
			return { matchId: null, status: "waiting" };
		},

		cancelSearch: (c) => {
			if (c.state.waitingConnId === c.conn.id) {
				c.state.waitingConnId = null;
			}
		},
	},
});

// Pong game room - actual gameplay
export const pongGame = actor({
	state: createInitialState(),

	createVars: () => {
		// Interval will be set in onWake
		return { gameInterval: null as ReturnType<typeof setInterval> | null };
	},

	onWake: (c) => {
		// Start the game loop
		c.vars.gameInterval = setInterval(() => {
			if (c.state.gameStarted) {
				updateGame(c as ActorContextOf<typeof pongGame>);
			}
		}, TICK_RATE);
	},

	onSleep: (c) => {
		if (c.vars.gameInterval) {
			clearInterval(c.vars.gameInterval);
		}
	},

	onConnect: (c, conn) => {
		if (!c.state.player1) {
			c.state.player1 = conn.id;
			c.broadcast("playerJoined", {
				player: "left",
				playersConnected: 1,
			});
		} else if (!c.state.player2) {
			c.state.player2 = conn.id;
			c.state.gameStarted = true;
			c.broadcast("gameStart", {
				ball: c.state.ball,
				leftPaddle: c.state.leftPaddle,
				rightPaddle: c.state.rightPaddle,
				score: c.state.score,
				gameStarted: true,
			});
		}
	},

	onDisconnect: (c, conn) => {
		if (c.state.player1 === conn.id) {
			c.state.player1 = null;
			c.state.gameStarted = false;
			c.broadcast("playerLeft", { player: "left" });
		} else if (c.state.player2 === conn.id) {
			c.state.player2 = null;
			c.state.gameStarted = false;
			c.broadcast("playerLeft", { player: "right" });
		}
	},

	actions: {
		setInput: (c, direction: "up" | "down" | null) => {
			// Determine which player based on connection
			if (c.state.player1 === c.conn.id) {
				c.state.leftInput = direction;
			} else if (c.state.player2 === c.conn.id) {
				c.state.rightInput = direction;
			}
		},

		getState: (c) => ({
			ball: c.state.ball,
			leftPaddle: c.state.leftPaddle,
			rightPaddle: c.state.rightPaddle,
			score: c.state.score,
			gameStarted: c.state.gameStarted,
		}),

		getPlayerAssignment: (c): PlayerSide | "spectator" | null => {
			if (c.state.player1 === c.conn.id) return "left";
			if (c.state.player2 === c.conn.id) return "right";
			if (c.state.player1 && c.state.player2) return "spectator";
			return null;
		},

		resetGame: (c) => {
			if (!c.state.gameStarted) return;
			const initial = createInitialState();
			c.state.ball = initial.ball;
			c.state.leftPaddle = initial.leftPaddle;
			c.state.rightPaddle = initial.rightPaddle;
			c.state.score = initial.score;
			c.state.leftInput = null;
			c.state.rightInput = null;
			// Keep players and gameStarted state
		},
	},
});

function createInitialState(): GameState {
	return {
		ball: {
			x: CANVAS_WIDTH / 2,
			y: CANVAS_HEIGHT / 2,
			vx: BALL_SPEED * (Math.random() > 0.5 ? 1 : -1),
			vy: BALL_SPEED * (Math.random() - 0.5) * 2,
		},
		leftPaddle: { y: CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2 },
		rightPaddle: { y: CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2 },
		score: { left: 0, right: 0 },
		leftInput: null,
		rightInput: null,
		player1: null,
		player2: null,
		gameStarted: false,
	};
}

function updateGame(c: ActorContextOf<typeof pongGame>) {
	const state = c.state;

	// Update paddle positions based on input
	if (state.leftInput === "up") {
		state.leftPaddle.y = Math.max(0, state.leftPaddle.y - PADDLE_SPEED);
	} else if (state.leftInput === "down") {
		state.leftPaddle.y = Math.min(
			CANVAS_HEIGHT - PADDLE_HEIGHT,
			state.leftPaddle.y + PADDLE_SPEED,
		);
	}

	if (state.rightInput === "up") {
		state.rightPaddle.y = Math.max(0, state.rightPaddle.y - PADDLE_SPEED);
	} else if (state.rightInput === "down") {
		state.rightPaddle.y = Math.min(
			CANVAS_HEIGHT - PADDLE_HEIGHT,
			state.rightPaddle.y + PADDLE_SPEED,
		);
	}

	// Update ball position
	state.ball.x += state.ball.vx;
	state.ball.y += state.ball.vy;

	// Ball collision with top/bottom walls
	if (state.ball.y <= 0 || state.ball.y >= CANVAS_HEIGHT - BALL_SIZE) {
		state.ball.vy = -state.ball.vy;
		state.ball.y = Math.max(
			0,
			Math.min(CANVAS_HEIGHT - BALL_SIZE, state.ball.y),
		);
	}

	// Ball collision with left paddle
	if (
		state.ball.x <= PADDLE_WIDTH + 20 &&
		state.ball.y + BALL_SIZE >= state.leftPaddle.y &&
		state.ball.y <= state.leftPaddle.y + PADDLE_HEIGHT &&
		state.ball.vx < 0
	) {
		state.ball.vx = -state.ball.vx * 1.05;
		state.ball.x = PADDLE_WIDTH + 20;
		const hitPos =
			(state.ball.y - state.leftPaddle.y) / PADDLE_HEIGHT - 0.5;
		state.ball.vy += hitPos * 3;
	}

	// Ball collision with right paddle
	if (
		state.ball.x >= CANVAS_WIDTH - PADDLE_WIDTH - 20 - BALL_SIZE &&
		state.ball.y + BALL_SIZE >= state.rightPaddle.y &&
		state.ball.y <= state.rightPaddle.y + PADDLE_HEIGHT &&
		state.ball.vx > 0
	) {
		state.ball.vx = -state.ball.vx * 1.05;
		state.ball.x = CANVAS_WIDTH - PADDLE_WIDTH - 20 - BALL_SIZE;
		const hitPos =
			(state.ball.y - state.rightPaddle.y) / PADDLE_HEIGHT - 0.5;
		state.ball.vy += hitPos * 3;
	}

	// Clamp ball velocity
	const maxVelocity = 15;
	state.ball.vx = Math.max(
		-maxVelocity,
		Math.min(maxVelocity, state.ball.vx),
	);
	state.ball.vy = Math.max(
		-maxVelocity,
		Math.min(maxVelocity, state.ball.vy),
	);

	// Score detection
	if (state.ball.x <= 0) {
		state.score.right += 1;
		resetBall(state);
	} else if (state.ball.x >= CANVAS_WIDTH) {
		state.score.left += 1;
		resetBall(state);
	}

	// Broadcast game state to all connected clients
	c.broadcast("gameState", {
		ball: state.ball,
		leftPaddle: state.leftPaddle,
		rightPaddle: state.rightPaddle,
		score: state.score,
		gameStarted: state.gameStarted,
	});
}

function resetBall(state: GameState) {
	state.ball.x = CANVAS_WIDTH / 2;
	state.ball.y = CANVAS_HEIGHT / 2;
	state.ball.vx = BALL_SPEED * (Math.random() > 0.5 ? 1 : -1);
	state.ball.vy = BALL_SPEED * (Math.random() - 0.5) * 2;
}

export const registry = setup({
	use: { matchmaker, pongGame },
});
