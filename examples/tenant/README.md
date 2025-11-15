# Tenant Dashboard for Rivet

Example project demonstrating multi-tenant organization management with role-based access control using [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js 18+

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/tenant
npm install
```

### Development

```sh
npm run dev
```

Open your browser to `http://localhost:3000`

## Features

- **Role-Based Access Control**: Different permissions for admin and member roles
- **Multi-Tenant Architecture**: Organization-scoped data and permissions
- **Authentication**: Token-based authentication with connection state
- **Real-time Updates**: Live updates when data changes across clients
- **Permission Enforcement**: Server-side permission checks for all operations
- **User Management**: Admin can add members and update roles
- **Invoice Management**: Admin-only access to billing information
- **Dashboard Analytics**: Role-specific statistics and insights

## How it works

This tenant system demonstrates:

1. **Authentication**: Token-based authentication with `createConnState`
2. **Authorization**: Role-based access control with server-side permission checks
3. **Multi-Tenancy**: Organization-scoped data isolation
4. **Real-time Collaboration**: Live updates across connected clients
5. **Permission Enforcement**: Different UI and API access based on user roles

## Architecture

- **Backend**: Rivet actor with authentication and role-based permissions
- **Frontend**: React application with conditional rendering based on user roles
- **Authentication**: Token-based with connection state for user context
- **Authorization**: Server-side permission checks for all sensitive operations

## User Roles

### Admin Users
- **Full Access**: Can view all data and perform all operations
- **Member Management**: Add new members and update member roles
- **Invoice Access**: View and manage organization invoices
- **Dashboard Stats**: Access to comprehensive analytics including revenue

### Member Users
- **Limited Access**: Can only view basic organization information
- **Member List**: View team members and their roles
- **Dashboard Stats**: Access to basic member statistics only
- **No Invoice Access**: Cannot view or manage billing information

### Data Isolation
- Organization-scoped data using actor keys
- User context stored in connection state
- Role-based data filtering and access control

## API Endpoints

### Public (All Authenticated Users)
- `getOrganization()` - Get organization information
- `getMembers()` - Get list of all members
- `getCurrentUser()` - Get current user information
- `getDashboardStats()` - Get basic statistics

### Admin Only
- `getInvoices()` - Get all invoices
- `addMember(member)` - Add new member
- `updateMemberRole(memberId, role)` - Update member role
- `markInvoicePaid(invoiceId)` - Mark invoice as paid

## Real-time Updates

The system broadcasts updates to all connected clients:

```typescript
// When member is added
c.broadcast("memberAdded", { member: newMember });

// When member role is updated
c.broadcast("memberUpdated", { member });

// When invoice is updated
c.broadcast("invoiceUpdated", { invoice });
```

## Use Cases

This tenant pattern is perfect for:

- **SaaS Applications**: Multi-tenant software with organization accounts
- **Team Management**: Internal tools with role-based access
- **Project Management**: Collaborative tools with permission levels
- **CRM Systems**: Customer relationship management with user roles
- **Enterprise Software**: Business applications with admin/user hierarchies
- **Learning Management**: Educational platforms with teacher/student roles

## Extending

This tenant system can be enhanced with:

### Advanced Authentication
- **OAuth Integration**: Google, GitHub, Microsoft authentication
- **JWT Tokens**: Stateless authentication with signed tokens
- **Multi-Factor Auth**: SMS, email, or authenticator app verification
- **Session Management**: Secure session handling and expiration

### Enhanced Authorization
- **Custom Roles**: Define custom roles beyond admin/member
- **Permissions**: Granular permissions for specific operations
- **Role Hierarchy**: Nested roles with inheritance
- **Resource-Level Access**: Per-resource permissions

### Multi-Tenancy Features
- **Organization Settings**: Configurable organization preferences
- **Billing Integration**: Stripe, PayPal, or other payment processors
- **Usage Tracking**: Monitor and limit resource usage per tenant
- **Data Export**: Allow tenants to export their data

### Advanced Features
- **Audit Logging**: Track all user actions and changes
- **Activity Feeds**: Real-time activity notifications
- **Team Invitations**: Invite users via email with signup flow
- **API Keys**: Generate API keys for external integrations
- **Webhooks**: Notify external systems of events

## Testing Different Roles

To test the role-based access control:

1. **Login as Alice (Admin)**:
   - Can view members and invoices
   - Can add new members
   - Can update member roles
   - Can mark invoices as paid
   - Sees full dashboard statistics

2. **Login as Bob/Charlie (Member)**:
   - Can view members only
   - Cannot access invoices
   - Cannot manage members
   - Sees limited dashboard statistics
   - Gets permission denied errors for admin operations

## Security Considerations

### Server-Side Validation
- All permission checks happen on the server
- Client-side UI is for user experience only
- Never trust client-side role information

### Token Management
- Use secure token storage (httpOnly cookies in production)
- Implement token refresh mechanisms
- Add token expiration and revocation

### Data Protection
- Sanitize all user inputs
- Use parameterized queries for database operations
- Implement rate limiting for API endpoints
- Log security events and failed authentication attempts

## Performance Considerations

### Caching
- Cache user roles and permissions
- Implement query result caching

### Scalability
- Separate read and write operations
- Use database read replicas for heavy read workloads
- Implement proper indexing for user and organization queries

## License

Apache 2.0
