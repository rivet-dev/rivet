# Onboarding States Reference

This document defines all possible states during the Rivet Cloud onboarding flow.

## User Context States

### 1. Anonymous (Unauthenticated)

**Description**: User is not logged in via Clerk

**User is shown**:
- Authentication page

**Available actions**:
- Log in with existing account
- Create new account

**Transitions To**:
- First-Time User (on sign up)
- Returning User (on login)

---

### 2. First-Time User

**Description**: User just completed sign-up, has no organization or project

**Automatic Actions**:
- Organization is created
- Project is created
- Default namespace is created

**User is shown**:
- Onboarding page

**Transitions To**:
- Onboarding: Path Selection

---

### 3. Returning User - No Projects

**Description**: User has organization but no projects

**Automatic Actions**:
- Project is created
- Default namespace is created

**User is shown**:
- Onboarding page

**Transitions To**:
- Onboarding: Path Selection

---

### 4. Returning User - No Namespaces

**Description**: User has project but no namespaces

**User is shown**:
- Onboarding page for the specific project

**Transitions To**:
- Onboarding: Path Selection

---

### 5. Returning User - No Runner Configs

**Description**: User has namespace but no runner configs

**User is shown**:
- Onboarding page for the specific namespace

**Transitions To**:
- Onboarding: Path Selection

---

### 6. Returning User - No Actors

**Description**: User has runner config but no actors (including destroyed)

**User is shown**:
- Onboarding page

**Transitions To**:
- Onboarding: Path Selection

---

### 7. Returning User - Has Actors

**Description**: User has at least one actor (including destroyed)

**User is shown**:
- Dashboard / Actors list view

---

## Routing Logic Summary

The system evaluates in order:

1. **Organization** → None? Create org, continue
2. **Projects** → 0? Create project + namespace → Onboarding
3. **Namespaces** → 0? → Onboarding
4. **Runner Configs** → 0? → Onboarding
5. **Actors (including destroyed)** → 0? → Onboarding, else → Dashboard

---

## Onboarding Flow States

### Path Selection

**Description**: Initial onboarding step where user chooses integration method

**Options Available**:
- Coding agent integration
- Template-based setup
- Manual integration

**Transitions To**:
- Template Selection (if template chosen)
- Create Project Form (if manual chosen)
- Integration Flow (if coding agent chosen)

---

### Template Selection

**Description**: User browses available templates

**User is shown**:
- List of templates
- Back navigation
- Start without template option

**Transitions To**:
- Path Selection (back)
- Template Detail (on template select)
- Create Project Form (on start without template)

---

### Template Detail

**Description**: User views template details and creates project

**User is shown**:
- Template information
- Project creation form
- Back navigation

**Transitions To**:
- Template Selection (back)
- Integration Flow: Provider Selection (on project create)

---

### Create Project Form

**Description**: User creates project without a template

**User is shown**:
- Project name input
- Form validation feedback

**Transitions To**:
- Path Selection (back, if from manual path)
- Template Selection (back, if from start without template)
- Integration Flow: Provider Selection (on project create)

---

### Integration Flow: Provider Selection

**Description**: User selects deployment provider

**User is shown**:
- Provider options (filtered by template if applicable)

**Transitions To**:
- Previous step (back)
- Integration Flow: Configuration (on provider select)

---

### Integration Flow: Configuration

**Description**: User configures deployment and validates endpoint

**User is shown**:
- Instructions (vary by flow type and provider)
- Environment variables
- Deployment endpoint input
- Validation status

**Blocked Until**:
- Valid URL format entered
- Endpoint validation succeeds

**On Proceed**:
- Runner config is created

**Transitions To**:
- Integration Flow: Provider Selection (back)
- Integration Flow: Verification (on submit)

---

### Integration Flow: Verification

**Description**: Waiting for Rivet Actor to be created

**User is shown**:
- Waiting status
- Manual actor creation action
- View deployment action (when template/provider supports frontend)

**State Persistence**:
- Survives page refresh
- Survives browser close/reopen

**Transitions To**:
- Dashboard (on actor detection)

---

### Dashboard

**Description**: Main application view with actors

**User is shown**:
- Celebration effect (only when coming from new user onboarding flow)
- Three-column layout
- Actor list and details

---

## State Persistence Rules

### Steps 1-2 (Provider Selection, Configuration)

| Action | Result |
|--------|--------|
| Page refresh | Reset to provider selection (template preserved) |
| Close/reopen | Reset to beginning |

### Step 3 (Verification)

| Action | Result |
|--------|--------|
| Page refresh | Stay on verification |
| Close/reopen | Return to verification |

---

## Flow-Specific Variations

### Manual Integration Flow

**Back Navigation from Configuration**:
- Returns to Provider Selection

**Back Navigation from Provider Selection**:
- Returns to Path Selection

---

### Template Flow

**Back Navigation from Configuration**:
- Returns to Provider Selection

**Back Navigation from Provider Selection**:
- Returns to Template Selection

**Back Navigation from Template Detail**:
- Returns to Template Selection

---

### Coding Agent Flow

**Back Navigation from Configuration**:
- Returns to Provider Selection

**Back Navigation from Provider Selection**:
- Returns to Path Selection

---

## Testing Checklist

- [ ] Verify anonymous user cannot access onboarding
- [ ] Verify first-time user sees onboarding after sign-up
- [ ] Verify organization, project, and namespace are auto-created for new users
- [ ] Verify user with namespace but no runner config triggers onboarding
- [ ] Verify user with runner config but no actors triggers onboarding
- [ ] Verify user with actors (including destroyed) sees dashboard
- [ ] Test all path selection options
- [ ] Verify template selection displays templates
- [ ] Verify start without template option works
- [ ] Test template detail page and back navigation
- [ ] Test back navigation at each step
- [ ] Verify state persistence rules for each step
- [ ] Test verification state persistence
- [ ] Verify dashboard loads after actor detection
- [ ] Verify celebration effect only for new user onboarding flow
