# Testing Documentation Guidelines

This document defines the rules and requirements for writing test scenarios and documentation for the Rivet Inspector frontend.

## Purpose

Test documentation should:
- Focus on **critical business logic** only
- Be **simple, readable, and maintainable**
- Avoid duplication through reference documents
- Be **ambiguous enough** to allow implementation flexibility
- Work for both **manual testing** and **e2e test development**

## Core Principles

### 1. Test What, Not How

✅ **Good**: "User is informed about connection failure"
❌ **Bad**: "Display error message: 'Connection failed. Please check your server is running at http://localhost:6420'"

**Why**: We care that users are informed, not the exact wording or format.

### 2. Avoid Specific Text or Formats

✅ **Good**: "Error details are shown"
❌ **Bad**: "Error message format: `Error: {message}`"

**Why**: UI text changes frequently. Test the presence of information, not exact formatting.

### 3. Focus on User Information and Actions

Every test should verify:
- **What the user sees/knows**: "User is informed about X"
- **What the user can do**: "Available actions: Retry, Cancel"

### 4. Business Logic Only

Include:
- ✅ Connection flows
- ✅ Rivet Actor lifecycle
- ✅ Data display and updates
- ✅ Error handling
- ✅ Critical user actions

Exclude:
- ❌ Accessibility testing
- ❌ Performance testing
- ❌ Visual styling
- ❌ Browser-specific quirks
- ❌ Animation timing

### 5. Use Reference Documents

When information is shared across multiple scenarios:
- Create a reference document in `references/`
- Link to it from main scenarios
- Avoid duplicating the same information

**Example**: Instead of repeating all Rivet Actor states in every test, reference `actor-states.md`.

## Document Structure

### Main Test Document Format

```markdown
# [Feature] Testing Scenarios

## Quick Reference
- Links to reference documents

## Testing Environment
- URLs, endpoints, prerequisites

## Critical User Flows

### Flow N: [Flow Name]

**Scenario**: Brief description

**Prerequisites** (if needed): What must be true before testing

**Verify**:
- Point 1
- Point 2

OR

**Test Cases**:

#### TCN.N: [Test Case Name]
**Given**: Initial conditions
**When**: User action
**Then**: Expected outcome (high-level)
```

### Reference Document Format

```markdown
# [Topic] Reference

## [Category]

### [Item Name]

**When**: Condition when this applies

**User is informed about**:
- Information point 1
- Information point 2

**Available actions**:
- Action 1
- Action 2

**Displays** (optional):
- UI element 1
- UI element 2
```

## Writing Style

### Language Rules

1. **Use present tense**: "User is informed" not "User will be informed"
2. **Be concise**: Avoid unnecessary words
3. **Be specific about intent**: "User is informed about connection failure" not "Show error"
4. **Avoid implementation details**: "Rivet Actor can be destroyed" not "Click destroy button and confirm in dialog"

### Verification Points

Use "Verify" sections for bullet-point checks:

```markdown
**Verify**:
- Feature works as expected
- Data is displayed
- Actions are available
```

Use "User is informed about" for error/information states:

```markdown
**User is informed about**:
- What went wrong
- How to fix it
```

Use "Available actions" for what users can do:

```markdown
**Available actions**:
- Retry connection
- Modify settings
```

### Scenario vs Test Case

**Use Scenario** for:
- High-level user flows
- Integration testing
- Multi-step processes

**Use Test Case** for:
- Specific conditions
- Edge cases
- Individual feature verification

## Naming Conventions

### Test Case IDs

Format: `TC[Flow].[Case]`

Examples:
- `TC1.1` - Flow 1, Test Case 1
- `TC5.14` - Flow 5, Test Case 14

### Integration Test IDs

Format: `I[Number]`

Examples:
- `I1` - Integration Test 1
- `I4` - Integration Test 4

### Edge Case IDs

Format: `E[Number]`

Examples:
- `E1` - Edge Case 1
- `E3` - Edge Case 3

### Reference Document Names

Use lowercase with hyphens:
- `connection-states.md`
- `actor-states.md`
- `error-states.md`
- `ui-components.md`

## Common Patterns

### Testing State Changes

✅ **Good**:
```markdown
**Verify**:
- Rivet Actor state updates when changed
- Events appear in real-time
```

❌ **Bad**:
```markdown
**Verify**:
- When actor state changes from {old} to {new}, the UI updates within 100ms showing the new state in JSON format
```

### Testing Error Handling

✅ **Good**:
```markdown
**When**: Connection fails
**User is informed about**:
- Connection failure
- Possible reasons

**Available actions**:
- Retry connection
```

❌ **Bad**:
```markdown
**When**: Connection fails
**Then**: Show red error banner with text "Connection failed: ERR_CONNECTION_REFUSED. Please check that your server is running." and a "Retry" button in blue
```

### Testing User Actions

✅ **Good**:
```markdown
**Verify**:
- Rivet Actor can be created
- Rivet Actor appears in list after creation
```

❌ **Bad**:
```markdown
**Steps**:
1. Click the "+" button next to "Game Session"
2. Fill in the form fields: name, region, max_players
3. Click "Create" button
4. Wait for success toast message
5. Verify actor appears in sidebar with green dot
```

## Examples

### Good Scenario Example

```markdown
### Flow 1: First-Time Visit

**Scenario**: User visits inspector URL without a running RivetKit server

**Verify**:
- Getting started card is displayed
- Connection form is shown with default endpoint
- User can enter custom endpoint
- Connect button is available
```

### Bad Scenario Example

```markdown
### Flow 1: First-Time Visit Without Server Running on Default Port

**Scenario**: When a user who has never used the inspector before opens the URL http://localhost:5173 in their Chrome browser and there is no RivetKit server running on port 6420

**Expected Behavior**:
- The page should render with a white background
- A card titled "Getting Started with Rivet Inspector" should appear centered on the screen
- Below that, a form with a single input field labeled "Endpoint" containing the placeholder text "http://localhost:6420" should be visible
- A blue button labeled "Connect" should be displayed below the input
- When hovering over the button, it should turn a darker shade of blue
```

## Checklist for New Test Documentation

Before submitting test documentation, verify:

- [ ] Focuses on business logic only (no accessibility, performance, styling)
- [ ] Uses high-level verification points
- [ ] Avoids specific UI text or message formats
- [ ] Uses reference documents for shared concepts
- [ ] Follows naming conventions
- [ ] Uses "User is informed about" for information verification
- [ ] Uses "Available actions" for action verification
- [ ] Uses "Verify" for feature verification
- [ ] Written in present tense
- [ ] Concise and readable
- [ ] No implementation details (no exact button clicks, colors, timing)
- [ ] Works for both manual and automated testing

## Updates and Maintenance

When updating test documentation:

1. **Keep it simple**: Remove details, don't add them
2. **Use references**: If adding new information that applies to multiple tests, create/update a reference document
3. **Stay high-level**: Focus on what users should know and do, not how the UI accomplishes it
4. **Remove outdated info**: Delete sections that no longer apply rather than commenting them out

## For AI Agents

When writing or updating test documentation:

1. Always check existing reference documents before creating new content
2. Default to high-level verification points
3. Never include specific UI text, error messages, or formatting
4. Focus on "User is informed about X" rather than "Error message says X"
5. Keep scenarios focused on single flows or features
6. Use bullet points for multiple verification items
7. Avoid adding notes, warnings, or future considerations
8. When in doubt, make it more ambiguous, not more specific
