# Multiplayer Game for Rivet

Example project demonstrating real-time multiplayer game mechanics with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/game
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Real-time Movement**: Smooth character movement with WASD/Arrow keys
- **Multiplayer Support**: Multiple players can join and move simultaneously
- **Visual Feedback**: Grid-based canvas with player identification
- **Collision Detection**: Players stay within game boundaries
- **Connection Status**: Live connection status indicator
- **Player Identification**: Current player highlighted in blue, others in gray

## How it works

This multiplayer game demonstrates:

1. **Real-time State Synchronization**: All players see the same game state in real-time
2. **Input Handling**: Client-side input captured and sent to server for processing
3. **Game Loop**: Server runs at 20 FPS (50ms intervals) to update game state
4. **Broadcasting**: World updates sent to all connected players
5. **Boundary Checking**: Players constrained to stay within the game world
6. **Player Management**: Automatic player creation/removal on connect/disconnect

## Architecture

- **Backend**: Rivet actor managing game state and player positions
- **Frontend**: React canvas-based game with real-time input handling
- **State Management**: Server-authoritative with client-side prediction
- **Networking**: WebSocket-based real-time communication

## Game Mechanics

### Movement System
- **Speed**: 5 pixels per frame (250 pixels/second)
- **Input**: Normalized directional input (-1, 0, 1)
- **Boundaries**: Players constrained to 10px margin from edges
- **Smoothness**: 50ms update intervals for responsive movement

### Player System
- **Spawning**: Random position within game boundaries
- **Identification**: Unique connection ID for each player
- **Visualization**: Blue circle for current player, gray for others
- **Cleanup**: Automatic removal when players disconnect

## Controls

- **W** or **↑**: Move up
- **A** or **←**: Move left  
- **S** or **↓**: Move down
- **D** or **→**: Move right

## Extending

This game can be extended with:

- **Combat System**: Player-to-player interactions
- **Power-ups**: Collectible items that affect gameplay
- **Obstacles**: Static or dynamic barriers in the game world
- **Teams**: Group players into competing teams
- **Scoring**: Points, levels, or achievement systems
- **Persistence**: Save player progress and statistics
- **Spectator Mode**: Watch games without participating
- **Game Modes**: Different rule sets (capture the flag, battle royale, etc.)
- **Enhanced Graphics**: Sprites, animations, and visual effects

## Performance Notes

- Game loop runs at 20 FPS for good balance of responsiveness and performance
- Input sampling at 20 FPS to match server tick rate
- Canvas rendering at 60 FPS for smooth visuals
- Optimized for up to 50 concurrent players per room

## License

Apache 2.0