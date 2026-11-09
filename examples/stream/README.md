# Stream Processor

Example project demonstrating real-time top-K stream processing.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/stream
npm install
npm run dev
```


## Features

- **Top-K Processing**: Maintains the top 3 highest values in real-time
- **Real-time Updates**: All connected clients see changes immediately
- **Stream Statistics**: Total count, highest value, and live metrics
- **Interactive Input**: Add custom values or generate random numbers
- **Reset Functionality**: Clear the stream and start fresh
- **Responsive Design**: Clean, modern interface with live statistics

## Implementation

This stream processor maintains the top 3 values in its SQLite database: each new value is inserted and everything below the top 3 is deleted in the same transaction. Updates are instantly sent to all connected clients via event broadcasting, and multiple users can add values simultaneously without overwriting each other.

- **Actor Definition** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/stream/src/index.ts)): Implements the `streamProcessor` actor, which keeps the value count and the top values in SQLite

## Resources

Read more about [state management](/docs/actors/state), [SQLite](/docs/actors/sqlite), [actions](/docs/actors/actions), and [events](/docs/actors/events).

## License

MIT
