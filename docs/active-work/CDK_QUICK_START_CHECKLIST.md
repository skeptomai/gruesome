# AWS CDK Quick Start Checklist

## Prerequisites (30 minutes)

- [ ] **Install Node.js**
  ```bash
  brew install node  # macOS
  node --version     # Verify v18+ or v20+
  ```

- [ ] **Install AWS CDK CLI**
  ```bash
  npm install -g aws-cdk
  cdk --version      # Should show 2.x.x
  ```

- [ ] **Install cargo-lambda**
  ```bash
  brew tap cargo-lambda/cargo-lambda
  brew install cargo-lambda
  cargo lambda --version
  ```

- [ ] **Configure AWS credentials** (if not already done)
  ```bash
  aws configure
  aws sts get-caller-identity  # Verify access
  ```

- [ ] **Bootstrap CDK** (one-time per account/region)
  ```bash
  # Get your AWS account ID
  export AWS_ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)

  # Bootstrap us-east-1 (required for CloudFront)
  cdk bootstrap aws://${AWS_ACCOUNT_ID}/us-east-1

  # Bootstrap your preferred backend region (e.g., us-west-2)
  cdk bootstrap aws://${AWS_ACCOUNT_ID}/us-west-2
  ```

---

## Phase 1: Initialize CDK Project

- [ ] **Create infrastructure directory**
  ```bash
  cd gruesome
  mkdir infrastructure
  cd infrastructure
  ```

- [ ] **Initialize CDK app**
  ```bash
  cdk init app --language typescript
  ```

- [ ] **Install CDK construct libraries**
  ```bash
  npm install aws-cdk-lib constructs
  ```

- [ ] **Copy stack files from implementation plan**
  - [ ] `lib/dns-stack.ts`
  - [ ] `lib/data-stack.ts`
  - [ ] `lib/auth-stack.ts`
  - [ ] `lib/backend-stack.ts`
  - [ ] `lib/frontend-stack.ts`
  - [ ] `bin/gruesome-platform.ts`

---

## Phase 2: Create Minimal Lambda Functions

- [ ] **Create Rust workspace**
  ```bash
  cd infrastructure
  mkdir -p lambda/gruesome-api
  cd lambda/gruesome-api

  # Create workspace Cargo.toml
  cat > Cargo.toml << 'EOF'
  [workspace]
  members = ["auth", "games", "saves"]
  EOF
  ```

- [ ] **Create auth function**
  ```bash
  cargo new --bin auth
  cd auth
  ```
  - [ ] Update `Cargo.toml` with Lambda dependencies
  - [ ] Copy placeholder `main.rs` from implementation plan
  - [ ] Test build: `cargo build`

- [ ] **Create games function**
  ```bash
  cd ../
  cargo new --bin games
  cd games
  ```
  - [ ] Update `Cargo.toml` with Lambda dependencies
  - [ ] Copy placeholder `main.rs`
  - [ ] Test build: `cargo build`

- [ ] **Create saves function**
  ```bash
  cd ../
  cargo new --bin saves
  cd saves
  ```
  - [ ] Update `Cargo.toml` with Lambda dependencies
  - [ ] Copy placeholder `main.rs`
  - [ ] Test build: `cargo build`

- [ ] **Build all functions for Lambda**
  ```bash
  cd ..  # Back to lambda/gruesome-api
  cargo lambda build --release --arm64
  ```
  - [ ] Verify artifacts in `target/lambda/auth/bootstrap`
  - [ ] Verify artifacts in `target/lambda/games/bootstrap`
  - [ ] Verify artifacts in `target/lambda/saves/bootstrap`

---

## Phase 3: Configure GitHub OAuth (Preparation)

- [ ] **Create GitHub OAuth App**
  1. Go to https://github.com/settings/developers
  2. Click "New OAuth App"
  3. Application name: `Gruesome Z-Machine Platform`
  4. Homepage URL: `https://gruesome.skeptomai.com`
  5. Callback URL: `https://gruesome.skeptomai.com/auth/callback`
  6. Register application

- [ ] **Save OAuth credentials**
  - [ ] Copy Client ID
  - [ ] Generate Client Secret
  - [ ] Store in password manager or AWS Secrets Manager

- [ ] **Update `lib/auth-stack.ts`**
  - [ ] Replace `YOUR_GITHUB_CLIENT_ID` with actual client ID
  - [ ] Replace `YOUR_GITHUB_CLIENT_SECRET` with actual secret
  - [ ] (Better: Use AWS Secrets Manager and `SecretValue.secretsManager()`)

---

## Phase 4: Deploy Infrastructure

- [ ] **Preview changes**
  ```bash
  cd infrastructure
  cdk diff
  ```
  - [ ] Review resources to be created
  - [ ] Verify no unexpected changes

- [ ] **Deploy DNS stack first** (creates certificate)
  ```bash
  cdk deploy GruesomeDnsStack
  ```
  - [ ] Wait for certificate DNS validation (~5-10 minutes)
  - [ ] Verify certificate status: ISSUED

- [ ] **Deploy data stack**
  ```bash
  cdk deploy GruesomeDataStack
  ```
  - [ ] Verify DynamoDB table created
  - [ ] Verify S3 saves bucket created

- [ ] **Deploy auth stack**
  ```bash
  cdk deploy GruesomeAuthStack
  ```
  - [ ] Verify Cognito User Pool created
  - [ ] Verify User Pool Client created

- [ ] **Deploy backend stack**
  ```bash
  cdk deploy GruesomeBackendStack
  ```
  - [ ] Verify Lambda functions deployed
  - [ ] Verify API Gateway created
  - [ ] Verify custom domain configured

- [ ] **Deploy frontend stack**
  ```bash
  cdk deploy GruesomeFrontendStack
  ```
  - [ ] Verify S3 frontend bucket created
  - [ ] Verify CloudFront distribution created (~15-30 minutes)
  - [ ] Verify Route 53 A record created

- [ ] **Verify all outputs**
  ```bash
  aws cloudformation describe-stacks --stack-name GruesomeFrontendStack \
    --query 'Stacks[0].Outputs'
  ```

---

## Phase 5: Verify Deployment

- [ ] **Test DNS resolution**
  ```bash
  dig gruesome.skeptomai.com
  dig api.gruesome.skeptomai.com
  ```
  - [ ] Both should resolve to AWS resources

- [ ] **Test SSL certificates**
  ```bash
  curl -I https://gruesome.skeptomai.com
  curl -I https://api.gruesome.skeptomai.com
  ```
  - [ ] Both should return 200 or valid response
  - [ ] Both should use valid SSL certificates

- [ ] **Test API endpoints**
  ```bash
  curl https://api.gruesome.skeptomai.com/api/auth/login
  ```
  - [ ] Should return placeholder JSON response

- [ ] **Test frontend**
  - [ ] Open https://gruesome.skeptomai.com in browser
  - [ ] Verify WASM loads (if deployed)
  - [ ] Check browser console for errors

---

## Phase 6: Next Steps

- [ ] **Implement real Lambda functions**
  - [ ] Auth: User registration, login, JWT generation
  - [ ] Games: List games, create game records
  - [ ] Saves: Upload/download save files to S3

- [ ] **Update WASM frontend**
  - [ ] Change API endpoint to `https://api.gruesome.skeptomai.com`
  - [ ] Add login UI
  - [ ] Implement save/load cloud integration

- [ ] **Configure monitoring**
  - [ ] Set up CloudWatch dashboards
  - [ ] Configure Lambda logs
  - [ ] Set up alarms for errors

- [ ] **Set up CI/CD**
  - [ ] Add GitHub Actions workflow
  - [ ] Auto-deploy on push to main
  - [ ] Run tests before deployment

---

## Troubleshooting

### Certificate Validation Stuck

**Symptoms**: DNS stack deploy hangs on certificate creation

**Solutions**:
- [ ] Verify Route 53 hosted zone exists for `skeptomai.com`
- [ ] Check NS records at domain registrar match Route 53 nameservers
- [ ] Wait up to 30 minutes for DNS propagation
- [ ] Check certificate status in ACM console

### Lambda Build Fails

**Symptoms**: `cargo lambda build` errors

**Solutions**:
- [ ] Verify Rust is installed: `rustc --version`
- [ ] Verify cargo-lambda installed: `cargo lambda --version`
- [ ] Check Cargo.toml dependencies are valid
- [ ] Run `cargo clean` and rebuild

### CDK Deploy Fails

**Symptoms**: Stack rollback or deployment errors

**Solutions**:
- [ ] Check AWS credentials: `aws sts get-caller-identity`
- [ ] Verify CDK is bootstrapped: `cdk bootstrap`
- [ ] Check stack dependencies in error message
- [ ] Review CloudFormation events in AWS console

### CloudFront Distribution Slow

**Symptoms**: Frontend stack takes 20-40 minutes

**Solutions**:
- [ ] This is normal - CloudFront deploys to all edge locations globally
- [ ] Monitor progress in CloudFormation console
- [ ] Distribution status should show "In Progress" then "Deployed"

### S3 Bucket Name Conflict

**Symptoms**: "BucketAlreadyExists" error

**Solutions**:
- [ ] S3 bucket names are globally unique
- [ ] Change `bucketName` in stacks to add unique suffix
- [ ] Or remove `bucketName` property to auto-generate

---

## Rollback / Cleanup

**To destroy specific stack**:
```bash
cdk destroy GruesomeFrontendStack
```

**To destroy all stacks**:
```bash
cdk destroy --all
```

**Note**: Stacks with `RETAIN` removal policy (DynamoDB, S3) will preserve data even after stack deletion. Delete manually if needed.

---

## Estimated Timeline

- Prerequisites setup: 30 minutes
- Phase 1 (Initialize CDK): 15 minutes
- Phase 2 (Create Lambda functions): 30 minutes
- Phase 3 (Configure GitHub OAuth): 10 minutes
- Phase 4 (Deploy infrastructure): 45-60 minutes
- Phase 5 (Verify deployment): 15 minutes

**Total**: ~2-3 hours for complete infrastructure deployment

---

## Success Criteria

- [ ] https://gruesome.skeptomai.com loads without errors
- [ ] https://api.gruesome.skeptomai.com/api/auth/login returns JSON
- [ ] Both domains have valid SSL certificates
- [ ] DynamoDB table exists with GSI
- [ ] Cognito User Pool created
- [ ] Lambda functions deployed
- [ ] S3 buckets created
- [ ] CloudFront distribution active
- [ ] All CDK stacks show "CREATE_COMPLETE" status

---

## Additional Resources

- **AWS CDK Documentation**: https://docs.aws.amazon.com/cdk/
- **CDK TypeScript API Reference**: https://docs.aws.amazon.com/cdk/api/v2/
- **cargo-lambda Documentation**: https://www.cargo-lambda.info/
- **Implementation Plan**: `docs/active-work/AWS_CDK_IMPLEMENTATION_PLAN.md`
- **Architecture Document**: `docs/active-work/MULTI_USER_PLATFORM_ARCHITECTURE.md`
