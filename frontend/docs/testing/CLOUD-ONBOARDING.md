# Cloud Onboarding Testing Scenarios

This document outlines critical testing scenarios for the Rivet Cloud onboarding frontend application.

## Quick Reference

- [Onboarding States Reference](./references/onboarding-states.md)
- [Integration Form Reference](./references/integration-form.md)
- [Actor States Reference](./references/actor-states.md)
- [UI Components Reference](./references/ui-components.md)

## Testing Environment

**Development URL**: http://localhost:5173 (or configured port)
**Production URL**: https://rivet.gg
**Authentication**: Clerk

---

## Authorization

### Anonymous User

**Scenario**: Unauthenticated user visits the application

**Verify**:
- User is redirected to authentication page
- Login and sign up options are available
- Protected routes are not accessible

---

## Critical User Flows

### Flow 1: Initial Login Routing

**Scenario**: User logs in and system determines where to route them based on account state

**Prerequisites**: User has successfully authenticated via Clerk

**Routing Logic** (evaluated in order):

1. **Organization Check**: Does user have an org?
   - No → Create default org automatically → Continue to step 2
   - Yes → Continue to step 2

2. **Project Check**: How many projects does the org have?
   - 0 → Create new project automatically with default namespace → Route to **New User Onboarding**
   - \>0 → Continue to step 3

3. **Namespace Check**: How many namespaces does the project have?
   - 0 → Route to **New User Onboarding**
   - \>0 → Continue to step 4

4. **Runner Config Check**: How many runner configs does the namespace have?
   - 0 → Route to **New User Onboarding**
   - \>0 → Continue to step 5

5. **Actor Check**: How many actors does the namespace have (including destroyed)?
   - 0 → Route to **New User Onboarding**
   - \>0 → Route to **Actors List View**

**Test Cases**:

#### TC1.1: New User - No Organization
**Given**: User has just completed sign-up with no existing org
**When**: Login completes
**Then**: Default org and project are created, user is routed to onboarding

#### TC1.2: User with Org but No Projects
**Given**: User has org but no projects
**When**: User navigates to org
**Then**: New project is created, user is routed to onboarding

#### TC1.3: User with Project but No Namespaces
**Given**: User has project with no namespaces
**When**: User navigates to project
**Then**: User is routed to onboarding

#### TC1.4: User with Namespace but No Runner Configs
**Given**: User has namespace with no runner configs
**When**: User navigates to namespace
**Then**: User is routed to onboarding

#### TC1.5: User with Runner Config but No Actors
**Given**: User has runner config but no actors (including destroyed)
**When**: User navigates to namespace
**Then**: User is routed to onboarding

#### TC1.6: User with Actors (Including Destroyed)
**Given**: User has at least one actor (can be destroyed)
**When**: User navigates to namespace
**Then**: User is routed to actors list view

#### TC1.7: Destroyed Actors Count Toward Total
**Given**: User has only destroyed actors (no running actors)
**When**: User navigates to namespace
**Then**: User is routed to actors list view (not onboarding)

---

### Flow 2: New User Onboarding - Path Selection

**Scenario**: User arrives at onboarding and chooses an integration path

**Prerequisites**: User was routed to onboarding based on Flow 1 logic

**Verify**:
- Three integration paths are available:
  - Coding agent integration
  - Template-based setup
  - Manual integration

**Test Cases**:

#### TC2.1: View Path Selection Options
**Given**: User is on onboarding
**When**: Page loads
**Then**: All three integration path options are displayed

#### TC2.2: Select Template Path
**Given**: User is on path selection
**When**: User selects template option
**Then**: User is navigated to templates list

#### TC2.3: Select Manual Integration Path
**Given**: User is on path selection
**When**: User selects manual integration option
**Then**: User is navigated to project creation form

#### TC2.4: Select Coding Agent Path
**Given**: User is on path selection
**When**: User selects coding agent option
**Then**: User proceeds to integration flow with coding agent context

---

### Flow 3: Template Selection

**Scenario**: User browses templates after selecting template path

**Prerequisites**: User selected template option from path selection

**Verify**:
- Available templates are displayed
- User can navigate back to path selection
- Option to start without a template is available
- Templates can be selected for details

**Test Cases**:

#### TC3.1: View Templates List
**Given**: User selected template path
**When**: Templates list loads
**Then**: Available templates are displayed

#### TC3.2: Navigate Back to Path Selection
**Given**: User is on templates list
**When**: User navigates back
**Then**: User returns to path selection

#### TC3.3: Start Without Template
**Given**: User is on templates list
**When**: User chooses to start without a template
**Then**: User proceeds to project creation form (without template context)

#### TC3.4: Select a Template
**Given**: User is on templates list
**When**: User selects a template
**Then**: User is navigated to template detail with project creation form

---

### Flow 4: Template Detail

**Scenario**: User views template details and creates project

**Prerequisites**: User selected a template from templates list

**Verify**:
- Template details are displayed
- Project creation form is available
- User can navigate back to templates list

**Test Cases**:

#### TC4.1: View Template Detail
**Given**: User selected a template
**When**: Template detail page loads
**Then**: Template information and project creation form are displayed

#### TC4.2: Navigate Back to Templates List
**Given**: User is on template detail
**When**: User navigates back
**Then**: User returns to templates list (not path selection)

#### TC4.3: Create Project with Template
**Given**: User is on template detail
**When**: User completes project form and submits
**Then**: Project is created with template context, user proceeds to integration flow

---

### Flow 5: Create Project Form (Manual Path)

**Scenario**: User creates project without a template

**Prerequisites**: User selected manual integration or started without a template

**Verify**:
- Project name input is available
- Form validates required fields
- Project is created with default namespace on submit

**Test Cases**:

#### TC5.1: View Create Project Form
**Given**: User selected manual integration or start without template
**When**: Form loads
**Then**: Project name input is displayed

#### TC5.2: Create Project Successfully
**Given**: User is on create project form
**When**: User enters valid project name and submits
**Then**: Project is created, user proceeds to integration flow

#### TC5.3: Project Name Validation
**Given**: User is on create project form
**When**: User enters invalid project name
**Then**: User is informed about validation error

---

### Flow 6: Integration Flow - Provider Selection

**Scenario**: After project creation, user selects a deployment provider

**Prerequisites**: User just created a project (from any path)

**Verify**:
- Available providers are displayed
- Provider list is filtered based on template (if applicable)
- User can select a provider to proceed

**Test Cases**:

#### TC6.1: View Provider Options After Project Creation
**Given**: User just created a project
**When**: Integration flow loads
**Then**: Available providers are displayed

#### TC6.2: Provider List Filtered by Template
**Given**: User created project with a template
**When**: Provider selection loads
**Then**: Only providers compatible with the selected template are shown

#### TC6.3: Provider List Shows All Options (No Template)
**Given**: User created project without a template
**When**: Provider selection loads
**Then**: All available providers are shown

#### TC6.4: Select Provider
**Given**: User is on provider selection
**When**: User selects a provider
**Then**: User proceeds to configuration step

---

### Flow 7: Configuration Step

**Scenario**: User sees configuration instructions based on their previous choices

**Prerequisites**: User selected a provider

**Configuration varies based on**:
- Previous integration path (template, coding agent, or manual)
- Whether provider supports one-click deploy

**Verify**:
- Configuration instructions are displayed based on context
- Environment variables are listed
- Deployment endpoint input is available
- User can navigate back to provider selection

**User is informed about**:
- How to set up their project (varies by path)
- Required environment variables
- Provider-specific guidance (when applicable)

**Available actions**:
- One-click deploy (when supported by provider and template)
- Navigate back to provider selection
- Enter deployment endpoint

**Test Cases**:

#### TC7.1: Template with One-Click Deploy
**Given**: User selected template path and provider supports one-click deploy
**When**: Configuration step loads
**Then**: One-click deploy option is available, setup instructions are displayed

#### TC7.2: Coding Agent with One-Click Deploy
**Given**: User selected coding agent path and provider supports one-click deploy
**When**: Configuration step loads
**Then**: Coding agent setup instructions are displayed

#### TC7.3: Template without One-Click Deploy
**Given**: User selected template path and provider does not support one-click deploy
**When**: Configuration step loads
**Then**: Manual setup instructions for template are displayed

#### TC7.4: Manual Integration
**Given**: User selected manual integration path
**When**: Configuration step loads
**Then**: Quick start guide instructions are displayed

#### TC7.5: Navigate Back from Configuration
**Given**: User is on configuration step
**When**: User navigates back
**Then**: User returns to provider selection

---

### Flow 8: Deployment Endpoint Validation

**Scenario**: User inputs deployment endpoint and system validates it

**Prerequisites**: User is on configuration step

**Validation Flow**:
1. User inputs deployment endpoint
2. System validates endpoint
3. If invalid → Show error, automatic re-validation occurs after user fixes
4. If valid → User can proceed

**Verify**:
- Endpoint field is required
- Invalid URL format is rejected
- Service availability is checked
- Re-validation occurs automatically after user fixes endpoint
- Proceed action only available after successful validation
- Runner config is created after validation succeeds

**User is informed about**:
- Validation errors (format or service unavailability)
- Successful validation status

**Test Cases**:

#### TC8.1: Empty Endpoint
**Given**: User is on configuration step
**When**: User attempts to proceed with empty endpoint
**Then**: User is informed about validation error

#### TC8.2: Invalid URL Format
**Given**: User is on configuration step
**When**: User enters invalid URL format
**Then**: User is informed about invalid format

#### TC8.3: Valid URL but Service Unavailable
**Given**: User enters valid URL format
**When**: Validation runs and service is not responding
**Then**: User is informed about service unavailability

#### TC8.4: Automatic Re-validation After Fix
**Given**: User saw validation error and fixed the endpoint
**When**: Some time passes
**Then**: Validation runs again automatically

#### TC8.5: Validation Success
**Given**: User enters valid endpoint with operational service
**When**: Validation succeeds
**Then**: User can proceed to next step

#### TC8.6: Runner Config Created on Success
**Given**: Endpoint validation succeeded
**When**: User proceeds
**Then**: Runner config is created

---

### Flow 9: Verification (Waiting for Actor)

**Scenario**: User waits for actor creation after completing configuration

**Prerequisites**: User completed configuration and runner config was created

**Verify**:
- Waiting status is displayed
- Manual actor creation option is available
- View deployment option available when template/provider supports frontend
- State persists on page refresh and close/reopen

**User is informed about**:
- Waiting status for actor creation

**Available actions**:
- Create actor manually
- View deployment (when frontend is supported)

**Test Cases**:

#### TC9.1: View Waiting State
**Given**: User completed configuration
**When**: Verification step loads
**Then**: User is informed about waiting for actor creation

#### TC9.2: View Deployment Available - Frontend Supported
**Given**: Runner config has template/provider that supports frontend
**When**: Verification step loads
**Then**: View deployment action is available

#### TC9.3: View Deployment Not Available - No Frontend Support
**Given**: Runner config template/provider does not support frontend
**When**: Verification step loads
**Then**: View deployment action is not available

#### TC9.4: Manual Actor Creation Available
**Given**: User is on verification step
**When**: Step loads
**Then**: Manual actor creation action is available

#### TC9.5: Create Actor via Manual Action
**Given**: User is on verification step
**When**: User chooses to create actor manually
**Then**: Actor creation interface is displayed

#### TC9.6: Actor Created in Background
**Given**: User is on verification step
**When**: Actor is created in background (by deployment)
**Then**: User is navigated to actors list view

#### TC9.7: Confetti Effect - From New User Onboarding
**Given**: User came from new user onboarding flow
**When**: Actor is detected and user transitions to dashboard
**Then**: Celebration effect is displayed

#### TC9.8: No Confetti - Existing User Flow
**Given**: User did not come from new user onboarding flow
**When**: Actor is detected and user transitions to dashboard
**Then**: No celebration effect

#### TC9.9: State Persists on Page Refresh
**Given**: User is on verification step
**When**: User refreshes the page
**Then**: User remains on verification step

#### TC9.10: State Persists on Close/Reopen
**Given**: User is on verification step
**When**: User closes and reopens the page
**Then**: User returns to verification step

---

### Flow 10: Dashboard / Actors List View

**Scenario**: User views the main dashboard after completing onboarding

**Prerequisites**: User has at least one actor (including destroyed)

**Verify**:
- Sidebar displays actor list and navigation
- Content area shows actors with their current state
- Details panel updates when actor is selected
- Organization and project switching is available

---

### Flow 11: Existing User Creates New Project

**Scenario**: User with existing organization creates a new project

**Prerequisites**: User is authenticated with at least one existing project/organization

**Verify**:
- User can navigate to new project creation
- Same path selection flow as new user onboarding
- New project is added to organization
- Integration flow is completed for new project

**Test Cases**:

#### TC11.1: Navigate to New Project Creation
**Given**: Existing user is in organization
**When**: User navigates to new project creation
**Then**: Path selection is displayed

#### TC11.2: Complete New Project Flow
**Given**: Existing user is on path selection for new project
**When**: User completes path selection, template/manual, and integration
**Then**: New project is created and configured

---

## Integration Test Scenarios

### I1: Complete New User Flow - Template with One-Click Deploy
**Verify**:
- New user signs up
- Org and project are auto-created
- User selects template path
- User selects template and creates project
- User selects provider with one-click support
- User completes configuration
- Runner config is created
- Actor is created
- Celebration effect is displayed
- User sees actors list

### I2: Complete New User Flow - Coding Agent
**Verify**:
- New user signs up
- Org and project are auto-created
- User selects coding agent path
- User selects provider
- User completes configuration
- Runner config is created
- Actor is created
- Celebration effect is displayed
- User sees actors list

### I3: Complete New User Flow - Template without One-Click Deploy
**Verify**:
- New user signs up
- User selects template path
- User selects template and provider without one-click support
- User completes manual setup configuration
- Runner config is created
- Actor is created
- User sees actors list

### I4: Complete New User Flow - Manual Integration
**Verify**:
- New user signs up
- User selects manual integration path
- User creates project
- User selects provider
- User completes configuration
- Runner config is created
- Actor is created
- User sees actors list

### I5: Existing User - Namespace with No Runner Config
**Verify**:
- Existing user has namespace but no runner configs
- User navigates to namespace
- User is routed to onboarding
- User completes integration flow
- Runner config is created
- Actor is created
- No celebration effect (not new user flow)

### I6: Existing User - Runner Config but No Actors
**Verify**:
- Existing user has runner config but no actors
- User navigates to namespace
- User is routed to onboarding verification step
- User creates actor
- No celebration effect (not new user flow)

### I7: Template Selection - Start Without Template
**Verify**:
- User selects template path
- User sees templates list
- User chooses to start without template
- User proceeds to project creation form without template context
- Integration shows manual configuration options

### I8: Endpoint Validation Retry Loop
**Verify**:
- User enters invalid endpoint
- User is informed about error
- User corrects endpoint
- Validation automatically re-runs
- On success, user can proceed

---

## Edge Cases

### E1: Organization Creation Failure
**Verify**: User is informed about failure and can retry

### E2: Project Creation Failure
**Verify**: User is informed about failure and can retry

### E3: Template Loading Failure
**Verify**: User is informed and can retry or choose different path

### E4: Provider Loading Failure
**Verify**: User is informed and can retry

### E5: Endpoint Validation Timeout
**Verify**: User is informed about timeout, automatic retry occurs

### E6: Actor Detection Timeout
**Verify**: User is informed, manual creation option remains available

### E7: Session Expiry During Onboarding
**Verify**: User is redirected to authentication, progress is preserved where possible

### E8: Network Disconnection During Validation
**Verify**: User is informed about network error, can retry when reconnected

### E9: Runner Config Creation Failure
**Verify**: User is informed, can retry

### E10: Destroyed Actor Only - Routing
**Verify**: User with only destroyed actors is routed to actors list (not onboarding)
