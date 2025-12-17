# Infrastructure Scripts

This directory contains scripts for verifying and testing the Gruesome game platform infrastructure.

## Scripts

### `verify-infrastructure.sh`

Verifies that all AWS infrastructure components are properly deployed and configured.

**Checks:**
- CloudFormation stacks status
- DynamoDB table existence and configuration (including TTL)
- S3 buckets and object counts
- Cognito User Pool and user counts
- Lambda functions (Auth and Game)
- API Gateway routes
- CloudFront distribution (optional)

**Usage:**
```bash
./scripts/verify-infrastructure.sh
```

**Requirements:**
- AWS CLI configured with valid credentials
- `jq` installed (`brew install jq`)
- Appropriate IAM permissions to describe resources

### `test-game-lambda.sh`

Runs end-to-end tests for the game-playing Lambda function.

**Tests:**
1. JWT authentication
2. Start new game (creates DynamoDB session)
3. Execute game command
4. Session persistence across multiple commands
5. Error handling (invalid session)
6. Session resume functionality
7. Health check endpoint

**Usage:**
```bash
./scripts/test-game-lambda.sh
```

**Environment Variables:**
- `API_URL` - API Gateway endpoint (default: production URL)
- `TEST_USERNAME` - Cognito username (default: "bob")
- `TEST_PASSWORD` - User password (default: "BobPassword123")
- `TEST_GAME_ID` - Game to test (default: "mini-zork")

**Example:**
```bash
# Test against production
./scripts/test-game-lambda.sh

# Test against custom endpoint
API_URL=https://custom-api.example.com ./scripts/test-game-lambda.sh
```

## Typical Workflow

After deploying infrastructure:

1. **Verify infrastructure is deployed correctly:**
   ```bash
   ./scripts/verify-infrastructure.sh
   ```

2. **Run end-to-end tests:**
   ```bash
   ./scripts/test-game-lambda.sh
   ```

3. **Check logs if tests fail:**
   ```bash
   aws logs tail /aws/lambda/GruesomeBackendStack-GameFunction* --follow
   ```

## Exit Codes

Both scripts use standard exit codes:
- `0` - All checks/tests passed
- `1` - One or more checks/tests failed

## Output

Scripts use colored output:
- ✓ Green - Passed
- ✗ Red - Failed
- ⚠ Yellow - Warning (non-critical)
- → Blue - Info

## Troubleshooting

### Infrastructure verification fails

**Issue:** CloudFormation stacks not found
- **Solution:** Deploy stacks with `npx cdk deploy --all`

**Issue:** Lambda functions not found
- **Solution:** Build and deploy Lambda functions:
  ```bash
  cd lambda/gruesome-api
  cargo lambda build --release --arm64 --package auth --package game
  npx cdk deploy GruesomeBackendStack
  ```

**Issue:** DynamoDB table exists but TTL disabled
- **Solution:** Enable TTL (script warns about this):
  ```bash
  aws dynamodb update-time-to-live \
    --table-name gruesome-platform \
    --time-to-live-specification "Enabled=true,AttributeName=ttl"
  ```

### End-to-end tests fail

**Issue:** Authentication fails
- **Solution:** Verify Cognito user exists:
  ```bash
  aws cognito-idp list-users --user-pool-id <pool-id>
  ```

**Issue:** Game start fails
- **Solution:** Check if game file exists in S3:
  ```bash
  aws s3 ls s3://gruesome-games/
  ```
- Upload if missing:
  ```bash
  aws s3 cp tests/mini_zork_release.z3 s3://gruesome-games/mini-zork.z3
  ```

**Issue:** Commands fail with "DynamoDB error"
- **Solution:** Check Lambda CloudWatch logs:
  ```bash
  aws logs tail /aws/lambda/GruesomeBackendStack-GameFunction* --since 5m
  ```

## CI/CD Integration

These scripts can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Verify Infrastructure
  run: ./infrastructure/scripts/verify-infrastructure.sh

- name: Run E2E Tests
  run: ./infrastructure/scripts/test-game-lambda.sh
  env:
    API_URL: ${{ secrets.API_URL }}
    TEST_USERNAME: ${{ secrets.TEST_USERNAME }}
    TEST_PASSWORD: ${{ secrets.TEST_PASSWORD }}
```

## Development

When adding new infrastructure components:

1. Add verification checks to `verify-infrastructure.sh`
2. Add integration tests to `test-game-lambda.sh`
3. Update this README with new checks/tests
4. Test locally before committing

## Related Documentation

- [Infrastructure README](../README.md) - CDK deployment instructions
- [Lambda README](../lambda/gruesome-api/README.md) - Lambda development
- [Game Service Documentation](../lambda/gruesome-api/game/README.md) - Game Lambda API
