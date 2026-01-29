# Development Workflow

## Local Development

### Setup

1. **Install dependencies:**
   ```bash
   cd frontend
   npm install
   ```

2. **Choose your API mode:**
   - **Production API** (default): Test against real backend at api.gruesome.skeptomai.com
   - **Mock API**: Test without backend (local mock server)

### Running Locally

#### Option A: Test with Production API (Recommended)

```bash
cd frontend
npm run dev
```

Then open http://localhost:3000

This uses the real production API. You can create test accounts and they'll be stored in production Cognito.

#### Option B: Test with Mock API

Terminal 1 - Start mock API server:
```bash
cd frontend
node mock-api.js
```

Terminal 2 - Start frontend with mock mode:
```bash
cd frontend
# Edit dev-config.js and set apiMode: 'mock'
npm run dev
```

**Mock API credentials:**
- Username: `testuser`
- Password: `TestPassword123`
- Password reset code: `123456`

### Live Reload

Browser-sync automatically reloads when you edit HTML, CSS, or JS files. Just save and the browser refreshes!

### Testing Checklist Before Deploy

Before deploying to staging or production, test locally:

- [ ] Login form works (shows username + password)
- [ ] Signup form works (shows email + username + password)
- [ ] Forgot password works (shows username only)
- [ ] Password reset confirmation works (shows username + code + new password)
- [ ] Toggle links switch modes correctly
- [ ] "Back to Login" returns to login mode from reset modes
- [ ] No browser console errors
- [ ] Form validation works (can't submit empty fields)
- [ ] Form validation errors don't appear for hidden fields

## Staging Environment

### Purpose

Staging is a complete replica of production for testing changes before deploying to users.

- **URL**: https://staging.gruesome.skeptomai.com
- **API**: https://api-staging.gruesome.skeptomai.com
- **Separate**: Own Cognito, DynamoDB, S3 buckets

### Deploy to Staging

**Frontend only:**
```bash
cd infrastructure
./scripts/deploy-frontend.sh staging
```

**Full stack (infrastructure + backend + frontend):**
```bash
cd infrastructure
npx cdk deploy --all --app "npx ts-node bin/gruesome-platform-staging.ts"
```

**Backend Lambda only:**
```bash
cd infrastructure/lambda/gruesome-api
cargo lambda build --release --arm64 --package auth
cd ../../
npx cdk deploy GruesomeBackendStackStaging --app "npx ts-node bin/gruesome-platform-staging.ts"
```

### Testing on Staging

1. Visit https://staging.gruesome.skeptomai.com
2. Create a test account (separate from production)
3. Test all functionality:
   - Login/Signup/Password Reset
   - Game loading
   - Save/Load
4. Check CloudWatch logs for errors
5. If everything works, deploy to production

## Production Deployment

### Frontend Only (Quick Updates)

```bash
cd infrastructure
./scripts/deploy-frontend.sh prod
```

### Full Stack

```bash
cd infrastructure
npx cdk deploy --all
```

### Backend Lambda Only

```bash
cd infrastructure/lambda/gruesome-api
cargo lambda build --release --arm64 --package auth
cd ../../
npx cdk deploy GruesomeBackendStack
```

## Workflow Summary

```
Local Dev (localhost:3000)
  ↓ Test & Verify
Staging (staging.gruesome.skeptomai.com)
  ↓ Test & Verify
Production (gruesome.skeptomai.com)
```

**Never skip staging!** Always test on staging before production.

## Common Issues

### "Required field" errors on hidden inputs

Make sure HTML inputs don't have hardcoded `required` attributes. JavaScript manages `required` dynamically.

### CORS errors in local dev

If using production API locally, make sure CORS is configured to allow `http://localhost:3000`.

### CloudFront cache issues

After deploying, wait 1-3 minutes for CloudFront invalidation. Use hard refresh (Cmd+Shift+R).

### Mock API not working

Make sure mock-api.js is running on port 3001 and dev-config.js has `apiMode: 'mock'`.

## Architecture

```
Local: localhost:3000 → Production API (or Mock API on :3001)
Staging: staging.gruesome.skeptomai.com → api-staging.gruesome.skeptomai.com
Production: gruesome.skeptomai.com → api.gruesome.skeptomai.com
```

Each environment is completely isolated with separate:
- Cognito User Pools
- DynamoDB tables
- S3 buckets
- Lambda functions
- CloudFront distributions
