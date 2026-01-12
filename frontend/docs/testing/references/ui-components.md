# UI Components Reference

This document defines the main UI components and their expected behavior in the Rivet Inspector.

## Layout Structure

### Three-Column Layout

```
┌──────────────┬───────────────────┬──────────────┐
│   Sidebar    │     Content       │   Details    │
│   (Left)     │     (Center)      │   (Right)    │
│              │                   │              │
│  - Connect   │  - Rivet Actor    │  - Selected  │
│    Info      │    List           │    Rivet     │
│  - Create    │  - Search/Filter  │    Actor     │
│    Actions   │  - Empty State    │    Info      │
│  - Footer    │                   │  - Tabs      │
│              │                   │  - Console   │
└──────────────┴───────────────────┴──────────────┘
```

**Responsive Behavior**: (if applicable)
- Mobile: Single column, collapsible panels
- Tablet: Two columns (sidebar collapses)
- Desktop: Full three columns

---

## Sidebar Components

### Connection Details

**When**: Inspector is connected to RivetKit server

**Displays**:
- RivetKit endpoint URL
- Connection status indicator
- Disconnect button (if supported)

---

### Rivet Actor Type Selector

**When**: Inspector is connected and ready to create Rivet Actors

**Displays**:
- List of available Rivet Actor types
- Create button/action for each type
- Rivet Actor type descriptions (optional)

**User Actions**:
- Click Rivet Actor type to open creation dialog
- View Rivet Actor type details

---

### Footer

**Displays**:
- Links to documentation
- Version information (optional)
- Support/help links

---

## Content Panel Components

### Rivet Actor List

**When**: Connected to RivetKit server

**Displays**:
- Each Rivet Actor as a list item
- Rivet Actor status indicator
- Rivet Actor keys
- Rivet Actor type (optional)

**States**:
- Empty: No Rivet Actors message
- Loading: Skeleton loader or spinner
- Populated: List of Rivet Actors

**User Actions**:
- Click Rivet Actor to view details in details panel
- Visual indication of selected Rivet Actor

---

### Search/Filter (Optional)

**Displays**:
- Search input field
- Filter dropdown/options

**Functionality**:
- Filter Rivet Actors by status
- Search Rivet Actors by key values
- Filter by Rivet Actor type

---

### Empty State

**When**: Connected but no Rivet Actors exist

**Displays**:
- No Rivet Actors message
- Call-to-action to create first Rivet Actor
- Help text about Rivet Actors

---

## Details Panel Components

### Rivet Actor Header

**Displays** (when Rivet Actor selected):
- Rivet Actor type name
- Rivet Actor status badge
- Destroy Rivet Actor button

---

### Status-Specific Content

See [Actor States Reference](./actor-states.md) for details on what displays for each state.

**Running + Connected**:
- Tab navigation
- Console accordion

**Running + Connecting**:
- Connecting message
- Loading indicator

**Running + Error**:
- Error message
- Retry button

**Sleeping**:
- Sleep state information
- Wake-up details/button

**Error**:
- Error details
- Error timestamp

---

### Tab Navigation

**When**: Rivet Actor is running and inspector is connected

**Tabs**:
1. State
2. Connections
3. Events
4. Metadata

**Behavior**:
- Click tab to switch view
- Active tab highlighted
- Tab content loads below

---

### Console Accordion

**When**: Rivet Actor is running and inspector is connected

**Location**: Bottom of details panel

**Components**:
- Available RPCs list
- Command input field
- Command execution button
- Command history/output

**User Actions**:
- Toggle accordion open/closed
- Select RPC from list
- Enter command in input
- Execute command

---

## Tab Content Components

### State Tab

**Displays**:
- Rivet Actor state data formatted as JSON or tree view
- Last updated timestamp (optional)
- Refresh button (optional)

---

### Connections Tab

**Displays**:
- Table/list of connected clients
- Connection metadata

**Information Shown**:
- Client ID
- Connected at timestamp
- Client metadata (if available)

---

### Events Tab

**Displays**:
- List of Rivet Actor events in chronological order
- Event type, message, timestamp

**Features**:
- Auto-scroll to latest (toggleable)
- Event filtering (optional)
- Clear events button (optional)

---

### Metadata Tab

**Displays**:
- Rivet Actor ID (copyable)
- Rivet Actor keys (copyable)
- Region information
- Additional metadata fields

---

## Getting Started Card

**When**: First visit, not connected to RivetKit server

**Displays**:
- Welcome message
- Quick start guide links
- Documentation links

---

## Connection Form

**When**: Not connected to RivetKit server

**Components**:
- Endpoint input field
- Connect button
- Help text/links

**Default Values**:
- Endpoint: `http://localhost:6420`

**Validation**:
- Endpoint must be valid URL
- Protocol required (http:// or https://)

---

## Common UI Patterns

### Loading States

**Spinner**: For short operations (< 2 seconds)
**Skeleton Loader**: For content loading
**Progress Bar**: For known-duration operations

---

### Status Badges

See [Actor States Reference](./actor-states.md) for badge styling.

**Format**: `[●] Status Text`

---

### Action Buttons

**Primary Actions**: Highlighted button (e.g., Connect, Create)
**Destructive Actions**: Red/warning styled (e.g., Destroy)
**Secondary Actions**: Subtle styling (e.g., Cancel, Disconnect)

---

## Testing Checklist

- [ ] Verify three-column layout displays correctly
- [ ] Test sidebar components render with correct data
- [ ] Verify Rivet Actor list displays and updates
- [ ] Test Rivet Actor selection updates details panel
- [ ] Verify tab switching works correctly
- [ ] Test console accordion expand/collapse
- [ ] Verify each tab displays correct content
- [ ] Test getting started card on first visit
- [ ] Verify connection form validation
- [ ] Test empty states display correctly
- [ ] Verify all user actions are clickable and functional
- [ ] Test copyable fields have copy functionality
