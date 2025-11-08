# Rate Limiter for Rivet

Example project demonstrating API rate limiting with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/rate
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Per-User Rate Limiting**: Each user/client gets independent rate limits
- **Sliding Window**: 5 requests per 60-second window
- **Real-time Status**: Live updates of remaining requests and reset time
- **Visual Progress**: Progress bar showing rate limit usage
- **Multiple Users**: Switch between users to test isolation
- **Admin Reset**: Reset rate limits for testing purposes

## How it works

This rate limiter demonstrates:

1. **Per-Actor Rate Limiting**: Each user gets their own actor instance with independent counters
2. **Time Window Management**: Automatic reset of counters when the time window expires
3. **Request Counting**: Track and limit the number of requests within the window
4. **Graceful Degradation**: Blocks requests when limits are exceeded
5. **Status Reporting**: Provide detailed information about current limits and reset times

## Architecture

- **Backend**: Rivet actor that maintains rate limit state per user
- **Frontend**: React application with real-time rate limit status
- **State Management**: Persistent rate limit counters with automatic window resets
- **User Isolation**: Each user/API client gets independent rate limiting

## Usage

1. Start the development server
2. Select a user from the dropdown
3. Click "Make API Request" to test the rate limiter
4. Watch the status update in real-time
5. Try making more than 5 requests within a minute to see blocking
6. Switch users to see independent rate limits
7. Use "Reset Rate Limiter" to clear limits for testing

## Rate Limiting Strategy

This example uses a **Fixed Window** approach:

- **Window Size**: 60 seconds
- **Request Limit**: 5 requests per window
- **Reset Behavior**: Counter resets to 0 when window expires
- **Granularity**: Per-user/client isolation

## Extending

This rate limiter can be extended with:

- Different rate limiting algorithms (sliding window, token bucket, etc.)
- Multiple rate limit tiers (basic/premium users)
- Geographic or IP-based limiting
- Dynamic rate limits based on user behavior
- Rate limit bypass for admin users
- Metrics and monitoring integration

## License

Apache 2.0
