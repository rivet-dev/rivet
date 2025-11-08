# Stream Processor for Rivet

Example project demonstrating real-time top-K stream processing with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/stream
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Top-K Processing**: Maintains the top 3 highest values in real-time
- **Real-time Updates**: All connected clients see changes immediately
- **Stream Statistics**: Total count, highest value, and live metrics
- **Interactive Input**: Add custom values or generate random numbers
- **Reset Functionality**: Clear the stream and start fresh
- **Responsive Design**: Clean, modern interface with live statistics

## How it works

This stream processor demonstrates:

1. **Top-K Algorithm**: Efficiently maintains the top 3 values using insertion sort
2. **Real-time Broadcasting**: Updates are instantly sent to all connected clients
3. **State Management**: Persistent tracking of values and statistics
4. **Event-driven Updates**: Live UI updates when new values are processed
5. **Collaborative Experience**: Multiple users can add values simultaneously

## Architecture

- **Backend**: Rivet actor managing stream state and top-K algorithm
- **Frontend**: React application with real-time stream visualization
- **State Management**: Server-side state with client-side event subscriptions
- **Algorithm**: Insertion-based top-K maintenance with O(k) complexity

## Stream Processing Algorithm

### Value Insertion
```typescript
// Insert new value maintaining sorted order
const insertAt = topValues.findIndex(v => value > v);
if (insertAt !== -1) {
    topValues.splice(insertAt, 0, value);
}

// Keep only top 3 values
if (topValues.length > 3) {
    topValues.length = 3;
}
```

### Performance Characteristics
- **Time Complexity**: O(k) per insertion where k=3
- **Space Complexity**: O(k) for storing top values
- **Memory Efficient**: Only stores top values, not entire stream
- **Real-time**: Sub-millisecond processing for new values

## Use Cases

This pattern is perfect for:

- **Leaderboards**: Gaming high scores, competition rankings
- **Metrics Monitoring**: Top error rates, highest traffic spikes
- **Social Features**: Most popular posts, trending content
- **Analytics Dashboards**: Key performance indicators
- **Real-time Alerts**: Threshold monitoring and notifications

## Extending

This stream processor can be enhanced with:

- **Configurable K**: Allow different top-K sizes (top 5, top 10, etc.)
- **Time Windows**: Top values within specific time periods
- **Multiple Streams**: Separate processors for different categories
- **Persistence**: Database storage for stream history
- **Complex Events**: Pattern detection and complex event processing
- **Aggregations**: Sum, average, and other statistical operations
- **Filters**: Value range filtering and validation
- **Rate Limiting**: Throttle input to prevent spam

## Stream Processing Concepts

### Top-K Algorithms
- **Heap-based**: Efficient for large K values
- **Sort-based**: Simple implementation for small K
- **Probabilistic**: Approximate results for massive streams

### Real-time Considerations
- **Latency**: Sub-millisecond processing requirements
- **Throughput**: Handling high-volume input streams
- **Memory**: Bounded memory usage regardless of stream size
- **Accuracy**: Exact vs. approximate results trade-offs

## Testing

The example includes basic structural tests. For production use, consider adding:

- **Algorithm correctness**: Verify top-K accuracy
- **Concurrency testing**: Multiple simultaneous inputs
- **Performance testing**: High-volume stream simulation
- **Edge cases**: Duplicate values, negative numbers, overflow handling

## License

Apache 2.0