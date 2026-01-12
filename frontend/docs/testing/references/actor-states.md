# Rivet Actor States Reference

This document defines all possible Rivet Actor states and their representations in the Inspector.

## State Definitions

### 1. Running

**Description**: Rivet Actor is actively running and processing

**UI Indicators**:
- Status badge: "Running" (typically green)
- Details panel shows full functionality
- All tabs available (State, Connections, Events, Metadata)
- Console accordion available

**Inspector Connection Sub-states**:
- Connecting to Inspector
- Connected to Inspector
- Connection Failed
- Outdated Version

**User Actions Available**:
- View all Rivet Actor details
- Execute commands via console
- Monitor real-time updates
- Destroy Rivet Actor

---

### 2. Sleeping

**Description**: Rivet Actor is in idle/sleep state to conserve resources

**UI Indicators**:
- Status badge: "Sleeping" (typically gray/muted)
- Limited details panel functionality
- Wake-up information displayed

**Sub-states**:
- Auto Wake-Up Enabled
- Auto Wake-Up Disabled

**User Actions Available**:
- View basic Rivet Actor information
- Destroy Rivet Actor
- Wake Rivet Actor (if manual wake-up available)

---

### 3. Error

**Description**: Rivet Actor encountered an error and stopped

**UI Indicators**:
- Status badge: "Error" (typically red)
- Error message/details displayed
- Limited panel functionality

**Error Information Displayed**:
- Error type/code
- Error message
- Timestamp of error
- Stack trace (if available)

**User Actions Available**:
- View error details
- Destroy Rivet Actor
- Restart Rivet Actor (if available)

---

### 4. Creating

**Description**: Rivet Actor is being initialized

**UI Indicators**:
- Status badge: "Creating" (typically yellow/pending)
- Loading indicator
- Minimal details available

**User Actions Available**:
- View creation progress
- Cancel creation (if supported)

**Transitions To**:
- Running (successful creation)
- Error (creation failed)

---

### 5. Destroying

**Description**: Rivet Actor is being shut down

**UI Indicators**:
- Status badge: "Destroying" (typically yellow/pending)
- Loading indicator
- Limited interactions

**User Actions Available**:
- Wait for destruction to complete

**Transitions To**:
- Removed from list (successful destruction)
- Error (destruction failed)

---

## Inspector Connection States (for Running Rivet Actors)

### Connected to Inspector

**Description**: Successfully connected to Rivet Actor's inspector interface

**UI Indicators**:
- Full details panel with all tabs
- Console accordion available
- Real-time updates working

**Available Features**:
- State tab: View Rivet Actor state
- Connections tab: View connected clients
- Events tab: View Rivet Actor events
- Metadata tab: View Rivet Actor metadata
- Console: Execute RPCs and commands

---

### Connecting to Inspector

**Description**: Attempting to connect to Rivet Actor's inspector

**UI Indicators**:
- "Connecting to inspector..." message
- Loading indicator
- Basic Rivet Actor status visible

**Available Features**:
- Rivet Actor status display
- Destroy Rivet Actor button

---

### Inspector Connection Failed

**Description**: Failed to connect to Rivet Actor's inspector

**UI Indicators**:
- Error message explaining connection failure
- Basic Rivet Actor status visible
- Retry option (if available)

**Possible Causes**:
- Rivet Actor doesn't have inspector enabled
- Network issues
- Inspector endpoint unreachable
- Authentication failure

**Available Features**:
- Rivet Actor status display
- Destroy Rivet Actor button
- Retry connection

---

### Outdated Inspector Version

**Description**: Rivet Actor's inspector version is incompatible

**UI Indicators**:
- Warning message about version mismatch
- Recommended action to update
- Basic Rivet Actor status visible

**Error Message**: "This Rivet Actor is running an outdated inspector version. Please update your RivetKit dependency to enable full inspector functionality."

**Available Features**:
- Rivet Actor status display
- Destroy Rivet Actor button
- Limited metadata view (if supported)

---

## Sleeping State Sub-types

### Sleeping (Auto Wake-Up Enabled)

**Description**: Rivet Actor will automatically wake up when needed

**UI Indicators**:
- "Sleeping (Auto Wake-Up Enabled)" status
- Information about auto wake-up behavior
- Expected wake-up conditions displayed

**Message Example**: "This Rivet Actor is sleeping and will automatically wake up when a client connects or an event is triggered."

---

### Sleeping (Auto Wake-Up Disabled)

**Description**: Rivet Actor requires manual wake-up

**UI Indicators**:
- "Sleeping (Manual Wake-Up Required)" status
- Information about manual wake-up
- Wake-up action button (if available)

**Message Example**: "This Rivet Actor is sleeping. Manual wake-up is required to activate it."

---

## State Transitions

```
Creating → Running
Creating → Error

Running → Sleeping
Running → Error
Running → Destroying

Sleeping → Running (wake-up)
Sleeping → Destroying

Error → Destroying
Error → Running (if restart supported)

Destroying → [Removed]
```

---

## Status Badge Styling Guidelines

Consistent visual representation of states:

| State | Color | Icon |
|-------|-------|------|
| Running | Green | ● or ▶ |
| Sleeping | Gray/Muted | ○ or ⏸ |
| Error | Red | ✕ or ⚠ |
| Creating | Yellow | ⟳ or … |
| Destroying | Orange | ⟳ or … |
| Connecting | Blue | ⟳ |

---

## Details Panel Content by State

### Running + Connected to Inspector
- ✅ All tabs visible
- ✅ Console accordion available
- ✅ Real-time updates
- ✅ Full metadata
- ✅ Destroy button

### Running + Connecting to Inspector
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Loading indicator
- ✅ Basic status
- ✅ Destroy button

### Running + Connection Failed
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Error message
- ✅ Retry option
- ✅ Destroy button

### Running + Outdated Version
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Warning message
- ⚠️ Limited metadata (if supported)
- ✅ Destroy button

### Sleeping (Auto Wake-Up)
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Wake-up info message
- ✅ Basic metadata
- ✅ Destroy button

### Sleeping (Manual Wake-Up)
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Manual wake-up message
- ✅ Wake-up button (if available)
- ✅ Destroy button

### Error
- ❌ Tabs hidden/disabled
- ❌ Console unavailable
- ✅ Error details
- ✅ Error timestamp
- ✅ Destroy button
- ⚠️ Restart button (if supported)

### Creating
- ❌ All features disabled
- ✅ Creation progress
- ⚠️ Cancel button (if supported)

### Destroying
- ❌ All features disabled
- ✅ Destruction progress

---

## Testing Checklist

- [ ] Verify each state displays correct status badge
- [ ] Verify appropriate UI elements shown/hidden for each state
- [ ] Test state transitions
- [ ] Verify error messages are clear
- [ ] Test inspector connection states for running Rivet Actors
- [ ] Verify sleeping states display correct information
- [ ] Test destroy action in each state
- [ ] Verify real-time state updates
- [ ] Test recovery from error states
- [ ] Verify console availability matches Rivet Actor state
