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

This stream processor uses a Top-K algorithm to efficiently maintain the top 3 values using insertion sort. Updates are instantly sent to all connected clients via event broadcasting. The actor maintains persistent state tracking values and statistics, and multiple users can add values simultaneously.

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/stream/src/backend/registry.ts)): Implements the `streamProcessor` actor with insertion-based Top-K maintenance with O(k) complexity for efficiently maintaining the highest values

## Resources

Read more about [state management](/docs/actors/state), [actions](/docs/actors/actions), and [events](/docs/actors/events).

## License

MIT
