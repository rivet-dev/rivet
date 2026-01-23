# Integration Form Reference

This document defines the integration form structure, validation rules, and content variations in the Rivet Cloud onboarding.

## Form Structure

### Step 1: Provider Selection

**Purpose**: User selects a deployment provider

**Components**:
- Provider list
- Provider selection action

**Validation**:
- Provider must be selected to proceed

**Provider List Behavior**:
- All providers shown for manual and coding agent flows
- Template-filtered list when template is selected

---

### Step 2: Configuration

**Purpose**: User configures deployment and validates endpoint

**Components**:
- Instructions section (varies by flow and provider)
- Environment variables list
- Deployment endpoint input
- Validation feedback
- Navigation controls

**Validation Rules**:
- Endpoint must be valid URL format
- Endpoint must pass API validation (service is operational)
- Automatic re-validation occurs after user fixes endpoint

**On Success**:
- Runner config is created when user proceeds

**Navigation**:
- Back returns to provider selection

---

### Step 3: Verification

**Purpose**: Wait for Rivet Actor creation and confirm deployment

**Components**:
- Waiting status
- Manual actor creation action
- View deployment action (conditional)

**View Deployment Condition**:
- Available when template and/or provider supports frontend

**Persistence**: State survives page refresh and browser close

---

## Content Variations by Flow

### Configuration Content Matrix

| Previous Choice | One-Click Deploy Support | Content Displayed |
|-----------------|--------------------------|-------------------|
| Template | Yes | One-click deploy option, setup instructions, env vars, endpoint input |
| Coding Agent | Yes | Coding agent setup instructions, env vars, endpoint input |
| Template | No | Clone instructions, setup instructions, provider guide, env vars, endpoint input |
| Manual | No | Quick start guide, setup instructions, provider guide, env vars, endpoint input |

---

### Manual Integration Flow

#### Step 2 Content

**User is informed about**:
- How to set up their project
- Required environment variables
- Provider-specific guidance

**Available actions**:
- Enter deployment endpoint
- Navigate back

---

### Coding Agent Flow

#### Step 2 Content

**User is informed about**:
- How to connect coding agent
- How to set up project with coding agent
- Required environment variables

**Available actions**:
- One-click deploy (when provider supports)
- Enter deployment endpoint
- Navigate back

---

### Template Flow

#### Step 2 Content

**User is informed about**:
- How to deploy template to provider
- Required environment variables
- Provider-specific guidance (when no one-click deploy)

**Available actions**:
- One-click deploy (when provider supports)
- Enter deployment endpoint
- Navigate back

---

## Provider-Specific Behavior

### Providers with One-Click Deploy

**Behavior**:
- Deploy action redirects to provider
- User completes deployment on provider
- User returns to continue form

---

### Providers without One-Click Deploy

**Behavior**:
- Clone/setup instructions are displayed
- Provider guide is available
- User manually deploys
- User enters deployment endpoint

---

## Validation States

### Endpoint Field States

#### Empty

**User is shown**:
- Empty input field

**Proceed action**: Disabled

---

#### Invalid Format

**When**: User enters non-URL value

**User is informed about**:
- Invalid URL format

**Proceed action**: Disabled

---

#### Valid Format, Validation Pending

**When**: User enters valid URL format, validation not started or pending

**User is shown**:
- Validation status indicator

**Proceed action**: Disabled

---

#### Validation In Progress

**When**: API validation is running

**User is shown**:
- Loading indicator

**Proceed action**: Disabled

---

#### Validation Failed

**When**: API validation returns error

**User is informed about**:
- Service unavailability or invalid response

**Behavior**:
- Automatic re-validation occurs after user fixes endpoint

**Proceed action**: Disabled

---

#### Validation Succeeded

**When**: API validation confirms service is operational

**User is shown**:
- Success indicator

**Proceed action**: Enabled

---

## Back Navigation Matrix

| Current Step | Previous Step (Manual) | Previous Step (Template) | Previous Step (Coding Agent) |
|--------------|------------------------|--------------------------|------------------------------|
| Provider Selection | Path Selection | Template Selection | Path Selection |
| Configuration | Provider Selection | Provider Selection | Provider Selection |
| Verification | N/A | N/A | N/A |

---

## State Persistence Summary

### On Page Refresh

| Step | State After Refresh |
|------|---------------------|
| Provider Selection | Reset to provider selection |
| Configuration | Reset to provider selection (template preserved) |
| Verification | Stay on verification |

### On Browser Close/Reopen

| Step | State After Reopen |
|------|-------------------|
| Provider Selection | Start from beginning |
| Configuration | Start from beginning |
| Verification | Return to verification |

---

## Testing Checklist

### Provider Selection
- [ ] Verify provider list displays
- [ ] Verify provider list varies by template
- [ ] Verify all providers shown for manual/coding agent flows
- [ ] Test provider selection proceeds to configuration
- [ ] Verify cannot proceed without selection
- [ ] Test back navigation for each flow type

### Configuration
- [ ] Verify instructions display correctly for template + one-click
- [ ] Verify instructions display correctly for coding agent + one-click
- [ ] Verify instructions display correctly for template + no one-click
- [ ] Verify instructions display correctly for manual
- [ ] Verify environment variables are listed
- [ ] Test endpoint validation - empty
- [ ] Test endpoint validation - invalid format
- [ ] Test endpoint validation - service unavailable
- [ ] Test endpoint validation - success
- [ ] Verify automatic re-validation after user fixes endpoint
- [ ] Verify proceed is disabled until validation succeeds
- [ ] Verify runner config is created on proceed
- [ ] Test back navigation

### Verification
- [ ] Verify waiting status displays
- [ ] Test manual actor creation action
- [ ] Test view deployment action when template supports frontend
- [ ] Test view deployment action when provider supports frontend
- [ ] Verify view deployment not available when no frontend support
- [ ] Verify state persists on refresh
- [ ] Verify state persists on close/reopen
- [ ] Verify transition to dashboard on actor detection
