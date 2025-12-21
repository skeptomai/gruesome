# Infrastructure Resource Mapping

**Complete mapping of all AWS resources across Production and Staging environments.**

Last verified: 2025-12-20

---

## Production Environment

### Frontend (Static Website)
- **Domain**: gruesome.skeptomai.com
- **S3 Bucket**: gruesome-frontend
- **CloudFront Distribution**: E36HKKVL2VZOZD
  - Domain: d1jj72oeonuyo5.cloudfront.net
  - Origin: gruesome-frontend.s3.us-east-1.amazonaws.com
- **CDK Stack**: GruesomeFrontendStack (us-east-1)
- **Route53**: A record pointing to CloudFront distribution

### Backend API
- **Domain**: api.gruesome.skeptomai.com
- **API Gateway**: viq8oiws3m.execute-api.us-west-1.amazonaws.com
- **CloudFront Distribution**: E2GRMKUTDD19Z6
  - Domain: d1dygu02p5bb2z.cloudfront.net
  - Origin: viq8oiws3m.execute-api.us-west-1.amazonaws.com
- **CDK Stack**: GruesomeBackendStack (us-west-1)
- **Route53**: A record pointing to CloudFront distribution

### Data Resources
- **DynamoDB Table**: gruesome-platform
- **S3 Saves Bucket**: gruesome-saves
- **S3 Games Bucket**: gruesome-games
- **CDK Stack**: GruesomeDataStack (us-west-1)

### Auth Resources
- **Cognito User Pool**: (production user pool)
- **CDK Stack**: GruesomeAuthStack (us-west-1)

---

## Staging Environment

### Frontend (Static Website)
- **Domain**: staging.gruesome.skeptomai.com
- **S3 Bucket**: gruesome-frontend-staging
- **CloudFront Distribution**: E1M8DHMS3GCUDX
  - Domain: d1w6h6wbkohqq2.cloudfront.net
  - Origin: gruesome-frontend-staging.s3.us-east-1.amazonaws.com
- **CDK Stack**: GruesomeFrontendStackStaging (us-east-1)
- **Route53**: A record pointing to CloudFront distribution

### Backend API
- **Domain**: api-staging.gruesome.skeptomai.com
- **API Gateway**: tihabkszgh.execute-api.us-west-1.amazonaws.com
- **CloudFront Distribution**: E3VWHUOBR5D259
  - Domain: d2jr9j1qj8xbwl.cloudfront.net
  - Origin: tihabkszgh.execute-api.us-west-1.amazonaws.com
- **CDK Stack**: GruesomeBackendStackStaging (us-west-1)
- **Route53**: A record pointing to CloudFront distribution

### Data Resources
- **DynamoDB Table**: gruesome-platform-staging
- **S3 Saves Bucket**: gruesome-saves-staging
- **S3 Games Bucket**: gruesome-games-staging
- **CDK Stack**: GruesomeDataStackStaging (us-west-1)

### Auth Resources
- **Cognito User Pool**: (staging user pool)
- **CDK Stack**: GruesomeAuthStackStaging (us-west-1)

---

## Shared Resources

### DNS
- **Hosted Zone**: skeptomai.com (Route53)
- **SSL Certificate**: arn:aws:acm:us-east-1:349145659387:certificate/33ae9627-b894-4edc-a480-201bc6e8b529
  - Covers: *.gruesome.skeptomai.com (wildcard)
  - Region: us-east-1 (required for CloudFront)
  - Managed: Manually (not in CloudFormation)
- **CDK Stack**: GruesomeDnsImportStack / GruesomeDnsImportStackStaging (import only)

---

## Quick Reference Table

| Resource Type | Production | Staging |
|---------------|-----------|---------|
| **Frontend Domain** | gruesome.skeptomai.com | staging.gruesome.skeptomai.com |
| **Frontend CloudFront** | E36HKKVL2VZOZD | E1M8DHMS3GCUDX |
| **Frontend S3** | gruesome-frontend | gruesome-frontend-staging |
| **API Domain** | api.gruesome.skeptomai.com | api-staging.gruesome.skeptomai.com |
| **API CloudFront** | E2GRMKUTDD19Z6 | E3VWHUOBR5D259 |
| **API Gateway** | viq8oiws3m | tihabkszgh |
| **DynamoDB** | gruesome-platform | gruesome-platform-staging |
| **Saves Bucket** | gruesome-saves | gruesome-saves-staging |
| **Games Bucket** | gruesome-games | gruesome-games-staging |

---

## Deployment Commands

### Frontend Deployment
```bash
# Production
./scripts/deploy-frontend.sh prod

# Staging
./scripts/deploy-frontend.sh staging
```

### Backend Deployment
```bash
# Build Lambda functions
cd lambda/gruesome-api
cargo lambda build --release --arm64

# Deploy via CDK
cd ../../
cdk deploy GruesomeBackendStack              # Production
cdk deploy GruesomeBackendStackStaging       # Staging
```

### CloudFront Invalidation
```bash
# Production frontend
aws cloudfront create-invalidation --distribution-id E36HKKVL2VZOZD --paths "/*"

# Staging frontend
aws cloudfront create-invalidation --distribution-id E1M8DHMS3GCUDX --paths "/*"

# Production API
aws cloudfront create-invalidation --distribution-id E2GRMKUTDD19Z6 --paths "/*"

# Staging API
aws cloudfront create-invalidation --distribution-id E3VWHUOBR5D259 --paths "/*"
```

---

## Important Notes

1. **Certificate**: The wildcard certificate `*.gruesome.skeptomai.com` is shared between production and staging. It must remain in `us-east-1` for CloudFront compatibility.

2. **Regional Separation**:
   - Frontend stacks are in `us-east-1` (CloudFront requirement)
   - Backend stacks are in `us-west-1` (Lambda/API Gateway)

3. **Complete Isolation**: Production and staging have completely separate:
   - S3 buckets
   - DynamoDB tables
   - Cognito user pools
   - API Gateways
   - CloudFront distributions
   - Only the SSL certificate and Route53 hosted zone are shared

4. **Deployment Script Bug**: The query in `deploy-frontend.sh` line 18 returns BOTH staging distributions (frontend + API) because both contain "staging.gruesome" in their aliases. This needs to be fixed to return only the frontend distribution.

---

## Verification Commands

```bash
# List all CloudFront distributions
aws cloudfront list-distributions --query 'DistributionList.Items[].{Id:Id,Alias:Aliases.Items[0]}' --output table

# List all API Gateways
aws apigatewayv2 get-apis --query 'Items[].{Name:Name,ApiId:ApiId,Endpoint:ApiEndpoint}' --output table

# Check CloudFront distribution details
aws cloudfront get-distribution --id E1M8DHMS3GCUDX --query 'Distribution.{DomainName:DomainName,Origins:Origins.Items[0].DomainName,Aliases:Aliases.Items}'

# Check what CloudFront is serving
curl -s https://staging.gruesome.skeptomai.com/app.js | head -20
```
