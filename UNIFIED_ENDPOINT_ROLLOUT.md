# Unified Endpoint Format Rollout

## Overview

This document describes the rollout of the unified endpoint format in Rivet UI and environment variables.

## What Changed

### Unified Endpoint Format

The unified endpoint format allows combining endpoint, namespace, and token into a single URL using HTTP Basic Auth syntax:

```
https://namespace:token@api.rivet.dev
```

This is more concise than the previous approach of using separate environment variables:

```bash
# Old format (still supported)
RIVET_ENDPOINT=https://api.rivet.dev
RIVET_NAMESPACE=namespace
RIVET_TOKEN=token

# New unified format (recommended)
RIVET_ENDPOINT=https://namespace:token@api.rivet.dev
```

### Changes Made

1. **UI Code Examples** (`frontend/src/app/publishable-token-code-group.tsx`)
   - JavaScript, React, and Next.js code examples now show the unified endpoint format
   - The endpoint URL automatically includes the namespace and token using URL encoding

2. **Environment Variables Component** (`frontend/src/app/env-variables.tsx`)
   - New `RivetEndpointEnvUnified` component that displays the unified endpoint format
   - Added `unified` prop (defaults to `true`) to toggle between formats
   - Backward compatibility maintained via the `unified` flag

3. **Tokens Page** (`frontend/src/routes/.../tokens.tsx`)
   - Added explanatory note about the unified endpoint format in the Client Token section

4. **Documentation** (`website/src/content/docs/self-hosting/connect-backend.mdx`)
   - Documented both unified and separate variable formats
   - Added comparison and recommendations
   - Updated all examples to show both approaches

## Backward Compatibility

✅ **Fully backward compatible** - Both formats are supported:

- Existing deployments using separate `RIVET_ENDPOINT`, `RIVET_NAMESPACE`, and `RIVET_TOKEN` variables will continue to work
- New deployments can use the unified format
- Users can choose the format that works best for their workflow

## Rollout Steps

### For UI Users

1. **View Updated Code Examples**
   - Navigate to the Tokens page in the Rivet dashboard
   - Code examples (JavaScript, React, Next.js) now show the unified endpoint format
   - Copy the provided code and use it in your application

2. **Environment Variables**
   - When viewing environment variables in the Connect dialogs, you'll see the unified `RIVET_ENDPOINT` format by default
   - The endpoint URL includes the namespace and token embedded in it

### For Documentation Users

1. **Read Updated Docs**
   - Visit `/docs/self-hosting/connect-backend` for detailed information
   - Both formats are documented with examples
   - Choose the format that fits your deployment workflow

### For Self-Hosted Users

No action required. Both formats are supported and parsed by RivetKit:

- The endpoint parser (`rivetkit-typescript/packages/rivetkit/src/utils/endpoint-parser.ts`) automatically extracts namespace and token from URLs
- Existing environment variable parsing continues to work as before

## Testing Recommendations

### Smoke Tests

1. **Client Token Flow**
   - [ ] Create a new namespace
   - [ ] View the Client Token section
   - [ ] Verify code examples show unified endpoint format
   - [ ] Copy code and test in a sample application

2. **Environment Variables**
   - [ ] Open Connect > Manual dialog
   - [ ] Verify environment variables show unified format
   - [ ] Copy and test in deployment

3. **Documentation**
   - [ ] Visit `/docs/self-hosting/connect-backend`
   - [ ] Verify both formats are documented
   - [ ] Try examples from documentation

### Integration Tests

1. **URL Parsing**
   - Test that `https://namespace:token@api.rivet.dev` correctly extracts:
     - endpoint: `https://api.rivet.dev/`
     - namespace: `namespace`
     - token: `token`

2. **Backward Compatibility**
   - Test that separate env vars still work:
     ```bash
     RIVET_ENDPOINT=https://api.rivet.dev
     RIVET_NAMESPACE=test
     RIVET_TOKEN=pub_test123
     ```

3. **Client Connection**
   - Test client creation with unified endpoint
   - Verify authentication works correctly

## Migration Guide (Optional)

Users can optionally migrate to the unified format:

### Before (Separate Variables)

```bash
RIVET_ENDPOINT=https://api-us-west-1.rivet.dev
RIVET_NAMESPACE=my-namespace
RIVET_TOKEN=pub_abc123xyz
```

### After (Unified Format)

```bash
RIVET_ENDPOINT=https://my-namespace:pub_abc123xyz@api-us-west-1.rivet.dev
```

**Note:** URL encoding is handled automatically by the URL API, but if setting this manually in shell scripts, ensure special characters are encoded.

## Benefits

1. **Simpler Configuration**: One variable instead of three
2. **Less Error-Prone**: No risk of mismatching endpoint/namespace/token
3. **Familiar Pattern**: Uses standard HTTP Basic Auth URL syntax
4. **Cleaner Code**: Client code is more concise

## Rollback Plan

If issues are discovered:

1. The `unified` prop in `EnvVariables` component can be set to `false` to revert to the old format
2. Documentation can be updated to recommend the separate format
3. UI code examples can be rolled back by reverting the commits

## Support

For questions or issues related to this rollout:

1. Check the updated documentation at `/docs/self-hosting/connect-backend`
2. Review the endpoint parser tests in `rivetkit-typescript/packages/rivetkit/src/utils/endpoint-parser.test.ts`
3. Open an issue on GitHub with the `FRONT-904` label

## Related Files

- `frontend/src/app/publishable-token-code-group.tsx` - Code examples component
- `frontend/src/app/env-variables.tsx` - Environment variables display
- `frontend/src/routes/.../tokens.tsx` - Tokens page
- `website/src/content/docs/self-hosting/connect-backend.mdx` - Documentation
- `rivetkit-typescript/packages/rivetkit/src/utils/endpoint-parser.ts` - URL parsing logic
