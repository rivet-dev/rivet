# Connection States Reference

This document defines all possible connection states in the Rivet Inspector.

## State Definitions

### 1. Disconnected

**Description**: No active connection to RivetKit server

**UI Indicators**:
- Connection form visible
- Getting started card displayed
- No Rivet Actor list visible

**User Actions Available**:
- Enter endpoint URL
- Click "Connect" button

**Transitions To**:
- Connecting
- Error (if validation fails)

---

### 2. Connecting

**Description**: Attempting to establish connection to RivetKit server

**UI Indicators**:
- Loading spinner/indicator
- "Connecting..." message
- Connect button disabled

**User Actions Available**:
- Cancel connection (optional)

**Transitions To**:
- Connected
- Connection Error

---

### 3. Connected

**Description**: Successfully connected to RivetKit server

**UI Indicators**:
- Three-column main app view visible
- Connection details in sidebar
- Rivet Actor list populated (or empty state)

**User Actions Available**:
- View Rivet Actors
- Create Rivet Actors
- Select Rivet Actors
- Disconnect (optional)

**Transitions To**:
- Disconnected (manual disconnect or network loss)
- Connection Error (network interruption)

---

### 4. Connection Error

**Description**: Failed to establish or maintain connection

**Sub-states**:
- Server Not Available
- Non-RivetKit Server
- CORS Error
- Network Error
- Local Network Access Denied

**UI Indicators**:
- Error message displayed
- Connection form visible
- Retry action available

**User Actions Available**:
- Modify endpoint URL
- Retry connection
- View troubleshooting help

**Transitions To**:
- Connecting (on retry)
- Disconnected (on cancel/clear)

---

## Connection Error Types

### Server Not Available

**When**: Cannot reach server at specified endpoint

**User is informed about**:
- Server is not reachable
- Should verify server is running

---

### Non-RivetKit Server

**When**: Server responds but is not a RivetKit server

**User is informed about**:
- Server is not a RivetKit server
- Should verify endpoint URL

---

### CORS Error

**When**: Server blocks request due to CORS policy

**User is informed about**:
- CORS configuration issue
- Chromium-based browser localhost issues
- CORS settings need checking

---

### Network Error

**When**: Network interruption during connection

**User is informed about**:
- Network error occurred
- Should retry connection

---

### Local Network Access Denied

**When**: Browser blocks access to local network

**User is informed about**:
- Local network access is denied
- Should allow access in browser settings

---

## Connection Persistence

### Auto-Reconnect Behavior

**Trigger**: Connection loss detected while in connected state

**Behavior**:
- Attempt automatic reconnection
- Display "Reconnecting..." indicator
- Limited retry attempts (e.g., 3 attempts)
- Fall back to connection error state after max retries

### Connection State Persistence

**Browser Refresh**:
- Attempt to reconnect to last successful endpoint
- Display connecting state during reconnection
- Fall back to disconnected state if reconnection fails

**URL Parameter**:
- `?u=<endpoint>` overrides stored endpoint
- Attempt connection on page load
- Store successful connection for future sessions

---

## Testing Checklist

- [ ] Verify disconnected state displays correctly
- [ ] Verify connecting state shows loading indicator
- [ ] Verify connected state loads main app view
- [ ] Test each connection error type
- [ ] Verify error messages are clear and actionable
- [ ] Test auto-reconnect on network interruption
- [ ] Test connection persistence after refresh
- [ ] Test URL parameter connection
- [ ] Test transition between all states
- [ ] Verify user cannot perform invalid actions in each state
