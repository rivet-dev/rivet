# Error States Reference

This document defines error states and their handling in the Rivet Inspector.

## Connection Errors

See [Connection States Reference](./connection-states.md) for connection error types.

**Types**:
- Server Not Available
- Non-RivetKit Server
- CORS Error
- Network Error
- Local Network Access Denied

---

## Rivet Actor Errors

### Rivet Actor Runtime Error

**When**: Rivet Actor encounters error during execution

**User is informed about**:
- Error occurred
- Basic error details
- Rivet Actor status

**Available actions**:
- Destroy Rivet Actor
- View error information

---

### Rivet Actor Creation Error

**When**: Failed to create new Rivet Actor

**User is informed about**:
- Creation failure
- Reason for failure

**Available actions**:
- Retry creation
- Modify configuration

---

### Rivet Actor Destruction Error

**When**: Failed to destroy Rivet Actor

**User is informed about**:
- Destruction failure

**Available actions**:
- Retry destruction
- Refresh Rivet Actor list

---

## Inspector Connection Errors

### Inspector Unreachable

**When**: Cannot connect to Rivet Actor's inspector interface

**User is informed about**:
- Connection failure
- Possible reasons

**Available actions**:
- Retry connection

---

### Inspector Version Mismatch

**When**: Rivet Actor inspector version is incompatible

**User is informed about**:
- Version incompatibility
- Need to update

**Available actions**:
- Continue with limited functionality

---

### Inspector Disconnected

**When**: Lost connection to Rivet Actor inspector

**User is informed about**:
- Connection loss
- Reconnection attempts

**Behavior**:
- Automatic reconnection attempts

**Available actions**:
- Wait for auto-reconnect
- Manually retry

---

## Console Errors

### RPC Execution Error

**When**: RPC call fails or returns error

**User is informed about**:
- RPC failure
- Error details from Rivet Actor

**Available actions**:
- Retry with corrected parameters

---

### Invalid Command

**When**: User enters invalid command in console

**User is informed about**:
- Command is invalid
- How to see available commands

**Available actions**:
- Correct command syntax
- View available commands

---

## Data Loading Errors

### State Load Error

**When**: Failed to load Rivet Actor state

**User is informed about**:
- State loading failure

**Available actions**:
- Retry loading

---

### Events Load Error

**When**: Failed to load Rivet Actor events

**User is informed about**:
- Events loading failure

**Available actions**:
- Retry loading

---

### Connections Load Error

**When**: Failed to load connection list

**User is informed about**:
- Connections loading failure

**Available actions**:
- Retry loading

---

## Error Recovery Patterns

### Auto-Retry
- Automatic retry attempts for temporary failures
- Eventually shows error state if retries fail

### Manual Retry
- User can retry failed operations
- Option to modify settings before retry

### Graceful Degradation
- Non-critical failures don't block other features
- Limited functionality available when full features unavailable

---

## Testing Checklist

- [ ] Test each connection error type displays correctly
- [ ] Verify Rivet Actor error states show proper information
- [ ] Test inspector connection errors
- [ ] Verify RPC execution errors display in console
- [ ] Test data loading errors in each tab
- [ ] Verify error messages are clear and actionable
- [ ] Test auto-retry behavior
- [ ] Test manual retry functionality
- [ ] Verify error recovery transitions to correct state
- [ ] Test multiple simultaneous errors
