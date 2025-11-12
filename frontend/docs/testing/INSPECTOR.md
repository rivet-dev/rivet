# Inspector Testing Scenarios

This document outlines critical testing scenarios for the Rivet Inspector frontend application.

## Quick Reference

- [Connection States Reference](./references/connection-states.md)
- [Error States Reference](./references/error-states.md)
- [Actor States Reference](./references/actor-states.md)
- [UI Components Reference](./references/ui-components.md)

## Testing Environment

**Development URL**: http://localhost:5173 (or configured port)
**Production URL**: https://inspect.rivet.dev
**Default RivetKit Endpoint**: http://localhost:6420

## Critical User Flows

### Flow 1: First-Time Visit (No RivetKit Server)

**Scenario**: User visits inspector URL without a running RivetKit server

**Steps**:
1. Navigate to inspector URL (local or production)
2. Verify getting started card is displayed
3. Verify connection form is displayed with:
   - Single input field labeled "Endpoint"
   - Input pre-filled with `http://localhost:6420`
   - "Connect" button

**Test Cases**:

#### TC1.1: Server Not Available
**Given**: No server running at the endpoint
**When**: User clicks "Connect"
**Then**: User is informed about server unavailability and CORS considerations

#### TC1.2: Non-RivetKit Server
**Given**: Server at endpoint is not a RivetKit server
**When**: User clicks "Connect"
**Then**: User is informed that server is not a RivetKit server

#### TC1.3: Local Network Access Not Granted
**Given**: Browser blocks local network access
**When**: User clicks "Connect"
**Then**: User is informed about local network access requirement

#### TC1.4: Successful Connection
**Given**: Valid RivetKit server at endpoint
**When**: User clicks "Connect"
**Then**: Navigate to main app view

---

### Flow 2: Direct Connection (RivetKit Server Available)

**Scenario**: User visits inspector URL with a running RivetKit server at default endpoint

**Steps**:
1. Start RivetKit server at `http://localhost:6420`
2. Navigate to inspector URL
3. Verify main app view loads immediately

**Expected**:
- No connection form displayed
- Auto-connection to default endpoint
- Main app view with three-column layout

---

### Flow 3: URL Parameter Connection

**Scenario**: User visits inspector URL with custom endpoint via query parameter

**Steps**:
1. Start RivetKit server at custom endpoint (e.g., `http://localhost:6420`)
2. Navigate to inspector URL with query parameter: `?u=http://localhost:6420`
3. Verify main app view loads with connection to specified endpoint

**Expected**:
- Auto-connection to endpoint specified in URL
- Main app view displays

---

### Flow 4: Main App View Navigation

**Scenario**: User interacts with the main app view components

See [UI Components Reference](./references/ui-components.md) for detailed component specifications.

**Layout**:
```
┌─────────────┬──────────────────┬─────────────┐
│   Sidebar   │     Content      │   Details   │
│             │                  │             │
└─────────────┴──────────────────┴─────────────┘
```

#### Sidebar (Inspector Mode)
**Displays**:
- Connection details
- Available Rivet Actor types to create
- Footer with links

**Test Cases**:
- TC4.1: Verify connection details match connected endpoint
- TC4.2: Verify Rivet Actor types list is populated
- TC4.3: Verify footer links are clickable

#### Content (Rivet Actor List)
**Displays**:
- List of currently running or sleeping Rivet Actors
- Each Rivet Actor shows its keys

**Test Cases**:
- TC4.4: Verify Rivet Actors are listed with correct states
- TC4.5: Verify Rivet Actor keys are displayed
- TC4.6: Verify clicking an Rivet Actor selects it and updates details panel

#### Details Panel
**Behavior**:
- Updates when an Rivet Actor is selected from content list
- Shows different states based on Rivet Actor status (see Flow 5)

---

### Flow 5: Details View States

**Scenario**: Details panel displays different content based on Rivet Actor state

See [Actor States Reference](./references/actor-states.md) for all possible states.

#### State 1: Sleeping Rivet Actor (Auto Wake-Up Enabled)
**Displays**:
- Wake-up information
- Rivet Actor status
- Destroy action

**Verify**:
- User is informed about auto wake-up behavior
- Destroy action is available

#### State 2: Sleeping Rivet Actor (Auto Wake-Up Disabled)
**Displays**:
- Sleeping information
- Rivet Actor status
- Destroy action

**Verify**:
- User is informed about manual wake-up requirement
- Manual wake-up action available (if supported)

#### State 3: Rivet Actor Error State
**Displays**:
- Error information
- Rivet Actor status
- Destroy action

**Verify**:
- User is informed about the error
- Error details are available

#### State 4: Inspector Connection Failed
**Displays**:
- Connection error information
- Rivet Actor status
- Destroy action

**Verify**:
- User is informed about connection failure
- Retry option available (if supported)

#### State 5: Connecting to Inspector
**Displays**:
- Connecting state
- Rivet Actor status
- Destroy action

**Verify**:
- Loading indicator shown
- Eventually transitions to connected or error state

#### State 6: Outdated Inspector Version
**Displays**:
- Version mismatch warning
- Rivet Actor status
- Destroy action

**Verify**:
- User is informed about version incompatibility
- Update suggestion provided

#### State 7: Successfully Connected
**Displays**:
- Rivet Actor status
- Tabs: State, Connections, Events, Metadata
- Console accordion
- Destroy action

**Verify**:
- All tabs are accessible
- Console accordion functions
- Full Rivet Actor details available

---

### Flow 6: Console Functionality

**Scenario**: User interacts with the console when connected to a running Rivet Actor

**Prerequisites**: Rivet Actor is running and inspector is successfully connected

**Verify**:
- Console accordion can be opened/closed
- Available RPCs are listed
- RPCs can be executed
- Command input field works
- Command output is displayed
- Errors are shown for failed commands

---

### Flow 7: Rivet Actor Details Tabs

**Scenario**: User navigates through Rivet Actor detail tabs

**Prerequisites**: Rivet Actor is running and inspector is successfully connected

#### Tab 1: State
**Verify**:
- Rivet Actor state data is displayed
- State updates reflect changes

#### Tab 2: Connections
**Verify**:
- Connected clients are listed
- Connection information is shown
- List updates when connections change

#### Tab 3: Events
**Verify**:
- Events are listed
- Events appear in order
- New events appear in real-time

#### Tab 4: Metadata
**Verify**:
- Rivet Actor ID is displayed
- Rivet Actor keys are shown
- Region information is available
- Metadata can be copied (if supported)

---

## Integration Test Scenarios

### I1: Full Connection Flow
**Verify**:
- Error shown when server unavailable
- Connection succeeds when server becomes available
- Main app view loads after successful connection

### I2: Rivet Actor Lifecycle
**Verify**:
- New Rivet Actor can be created
- Rivet Actor appears in list after creation
- Rivet Actor details can be viewed
- Rivet Actor can be destroyed
- Rivet Actor is removed from list after destruction

### I3: Real-Time Updates
**Verify**:
- State changes are reflected in details view
- Events appear in real-time
- Connections list updates when clients connect/disconnect

### I4: Multi-Rivet Actor Management
**Verify**:
- Multiple Rivet Actors can be created
- All Rivet Actors appear in list
- Can switch between Rivet Actors in details panel
- Each Rivet Actor shows its own unique data

---

## Edge Cases and Error Scenarios

See [Error States Reference](./references/error-states.md) for comprehensive error scenarios.

### E1: Network Interruption
**Verify**: Connection loss is handled and reconnection is attempted

### E2: RivetKit Server Restart
**Verify**: Disconnection is detected and auto-reconnect works

### E3: Browser Refresh
**Verify**: Connection can be re-established after refresh

### E4: Invalid Endpoint Format
**Verify**: Invalid URLs are rejected with error message

### E5: CORS Errors
**Verify**: CORS issues show informative error message
