# AWS CDK Implementation Plan: Multi-User Platform

## Overview

Automated deployment of the complete multi-user Z-Machine platform using AWS CDK (TypeScript) with zero manual AWS console configuration.

**Goal**: Single command deployment of entire infrastructure: `cdk deploy --all`

**Domain**: `gruesome.skeptomai.com` (frontend) + `api.gruesome.skeptomai.com` (backend)

**Existing**: Route 53 hosted zone for `skeptomai.com` already exists

---

## Prerequisites

### 1. Install Required Tools

```bash
# Install Node.js (for CDK)
brew install node  # macOS
# or download from nodejs.org

# Install AWS CDK CLI
npm install -g aws-cdk

# Install cargo-lambda (for building Rust Lambda functions)
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda

# Verify installations
cdk --version          # Should show 2.x.x
cargo lambda --version # Should show latest version
node --version         # Should show v18+ or v20+
```

### 2. Configure AWS Credentials

```bash
# If not already configured
aws configure

# Verify access
aws sts get-caller-identity

# Should show your AWS account ID and user/role
```

### 3. Bootstrap CDK (One-Time Setup per AWS Account/Region)

```bash
# Bootstrap CDK in your AWS account for us-east-1 (required for CloudFront)
cdk bootstrap aws://ACCOUNT-ID/us-east-1

# If using different region for backend, bootstrap that too
cdk bootstrap aws://ACCOUNT-ID/us-west-2  # example
```

---

## Project Structure

Create new directory structure within the repository:

```
gruesome/
├── infrastructure/              # NEW: CDK infrastructure
│   ├── bin/
│   │   └── gruesome-platform.ts
│   ├── lib/
│   │   ├── dns-stack.ts
│   │   ├── frontend-stack.ts
│   │   ├── backend-stack.ts
│   │   ├── auth-stack.ts
│   │   └── data-stack.ts
│   ├── lambda/
│   │   └── gruesome-api/        # Rust workspace for Lambda functions
│   │       ├── Cargo.toml
│   │       ├── auth/
│   │       │   ├── Cargo.toml
│   │       │   └── src/
│   │       │       └── main.rs
│   │       ├── games/
│   │       │   ├── Cargo.toml
│   │       │   └── src/
│   │       │       └── main.rs
│   │       └── saves/
│   │           ├── Cargo.toml
│   │           └── src/
│   │               └── main.rs
│   ├── cdk.json
│   ├── package.json
│   ├── tsconfig.json
│   └── README.md
├── wasm/                        # Existing WASM frontend
│   └── ...
└── src/                         # Existing interpreter source
    └── ...
```

---

## Implementation Steps

### Step 1: Initialize CDK Project

```bash
cd gruesome
mkdir infrastructure
cd infrastructure

# Initialize CDK app with TypeScript
cdk init app --language typescript

# Install additional CDK construct libraries
npm install @aws-cdk/aws-certificatemanager \
            @aws-cdk/aws-cloudfront \
            @aws-cdk/aws-cloudfront-origins \
            @aws-cdk/aws-route53 \
            @aws-cdk/aws-route53-targets \
            @aws-cdk/aws-s3 \
            @aws-cdk/aws-s3-deployment \
            @aws-cdk/aws-dynamodb \
            @aws-cdk/aws-cognito \
            @aws-cdk/aws-apigatewayv2 \
            @aws-cdk/aws-apigatewayv2-integrations \
            @aws-cdk/aws-lambda \
            aws-cdk-lib constructs
```

### Step 2: Define Infrastructure Stacks

Create the following stack files in `lib/`:

#### 2.1: DNS Stack (`lib/dns-stack.ts`)

```typescript
import * as cdk from 'aws-cdk-lib';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import { Construct } from 'constructs';

export class DnsStack extends cdk.Stack {
  public readonly hostedZone: route53.IHostedZone;
  public readonly certificate: acm.Certificate;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // Import existing hosted zone for skeptomai.com
    this.hostedZone = route53.HostedZone.fromLookup(this, 'HostedZone', {
      domainName: 'skeptomai.com',
    });

    // Create wildcard certificate for *.gruesome.skeptomai.com
    // MUST be in us-east-1 for CloudFront
    this.certificate = new acm.Certificate(this, 'GruesomeCertificate', {
      domainName: '*.gruesome.skeptomai.com',
      subjectAlternativeNames: ['gruesome.skeptomai.com'],
      validation: acm.CertificateValidation.fromDns(this.hostedZone),
    });

    // Output certificate ARN
    new cdk.CfnOutput(this, 'CertificateArn', {
      value: this.certificate.certificateArn,
      description: 'ACM Certificate ARN for gruesome.skeptomai.com',
    });
  }
}
```

#### 2.2: Data Stack (`lib/data-stack.ts`)

```typescript
import * as cdk from 'aws-cdk-lib';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';
import { Construct } from 'constructs';

export class DataStack extends cdk.Stack {
  public readonly table: dynamodb.Table;
  public readonly savesBucket: s3.Bucket;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // DynamoDB table with single-table design
    this.table = new dynamodb.Table(this, 'GruesomePlatformTable', {
      tableName: 'gruesome-platform',
      partitionKey: {
        name: 'PK',
        type: dynamodb.AttributeType.STRING,
      },
      sortKey: {
        name: 'SK',
        type: dynamodb.AttributeType.STRING,
      },
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST, // On-demand pricing
      pointInTimeRecovery: true, // Automatic backups
      removalPolicy: cdk.RemovalPolicy.RETAIN, // Don't delete on stack destroy
    });

    // Global Secondary Index for email lookup
    this.table.addGlobalSecondaryIndex({
      indexName: 'email-index',
      partitionKey: {
        name: 'email',
        type: dynamodb.AttributeType.STRING,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // S3 bucket for save files
    this.savesBucket = new s3.Bucket(this, 'GruesomeSavesBucket', {
      bucketName: 'gruesome-saves',
      versioned: true, // Keep save file history
      lifecycleRules: [
        {
          // Delete old versions after 90 days
          noncurrentVersionExpiration: cdk.Duration.days(90),
        },
      ],
      cors: [
        {
          allowedOrigins: ['https://gruesome.skeptomai.com'],
          allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT, s3.HttpMethods.DELETE],
          allowedHeaders: ['*'],
        },
      ],
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // Outputs
    new cdk.CfnOutput(this, 'TableName', {
      value: this.table.tableName,
    });
    new cdk.CfnOutput(this, 'SavesBucketName', {
      value: this.savesBucket.bucketName,
    });
  }
}
```

#### 2.3: Auth Stack (`lib/auth-stack.ts`)

```typescript
import * as cdk from 'aws-cdk-lib';
import * as cognito from 'aws-cdk-lib/aws-cognito';
import { Construct } from 'constructs';

export class AuthStack extends cdk.Stack {
  public readonly userPool: cognito.UserPool;
  public readonly userPoolClient: cognito.UserPoolClient;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // Cognito User Pool
    this.userPool = new cognito.UserPool(this, 'GruesomeUserPool', {
      userPoolName: 'gruesome-users',
      selfSignUpEnabled: true,
      signInAliases: {
        email: true,
        username: true,
      },
      autoVerify: {
        email: true,
      },
      passwordPolicy: {
        minLength: 8,
        requireLowercase: true,
        requireUppercase: true,
        requireDigits: true,
        requireSymbols: false,
      },
      accountRecovery: cognito.AccountRecovery.EMAIL_ONLY,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // User Pool Client for web app
    this.userPoolClient = this.userPool.addClient('WebClient', {
      authFlows: {
        userPassword: true,
        userSrp: true,
      },
      oAuth: {
        flows: {
          authorizationCodeGrant: true,
        },
        scopes: [
          cognito.OAuthScope.EMAIL,
          cognito.OAuthScope.OPENID,
          cognito.OAuthScope.PROFILE,
        ],
        callbackUrls: [
          'https://gruesome.skeptomai.com/auth/callback',
          'http://localhost:8080/auth/callback', // For local development
        ],
      },
    });

    // GitHub OAuth identity provider (requires GitHub OAuth app)
    // NOTE: You'll need to create GitHub OAuth app and get client ID/secret
    // Then store secret in AWS Secrets Manager and reference here
    const githubProvider = new cognito.UserPoolIdentityProviderOidc(this, 'GitHubProvider', {
      userPool: this.userPool,
      name: 'GitHub',
      clientId: 'YOUR_GITHUB_CLIENT_ID', // TODO: Replace or use SecretValue
      clientSecret: 'YOUR_GITHUB_CLIENT_SECRET', // TODO: Use SecretValue
      issuerUrl: 'https://github.com',
      attributeMapping: {
        email: cognito.ProviderAttribute.other('email'),
        preferredUsername: cognito.ProviderAttribute.other('login'),
      },
      scopes: ['user:email', 'read:user'],
    });

    // Outputs
    new cdk.CfnOutput(this, 'UserPoolId', {
      value: this.userPool.userPoolId,
    });
    new cdk.CfnOutput(this, 'UserPoolClientId', {
      value: this.userPoolClient.userPoolClientId,
    });
  }
}
```

#### 2.4: Backend Stack (`lib/backend-stack.ts`)

```typescript
import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigatewayv2 from 'aws-cdk-lib/aws-apigatewayv2';
import * as apigatewayv2_integrations from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as route53_targets from 'aws-cdk-lib/aws-route53-targets';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as cognito from 'aws-cdk-lib/aws-cognito';
import { Construct } from 'constructs';

interface BackendStackProps extends cdk.StackProps {
  table: dynamodb.Table;
  savesBucket: s3.Bucket;
  userPool: cognito.UserPool;
  hostedZone: route53.IHostedZone;
  certificate: acm.Certificate;
}

export class BackendStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: BackendStackProps) {
    super(scope, id, props);

    // Lambda function for auth endpoints
    const authFunction = new lambda.Function(this, 'AuthFunction', {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('../lambda/gruesome-api/target/lambda/auth'),
      environment: {
        TABLE_NAME: props.table.tableName,
        USER_POOL_ID: props.userPool.userPoolId,
      },
      timeout: cdk.Duration.seconds(30),
    });

    // Lambda function for games endpoints
    const gamesFunction = new lambda.Function(this, 'GamesFunction', {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('../lambda/gruesome-api/target/lambda/games'),
      environment: {
        TABLE_NAME: props.table.tableName,
      },
      timeout: cdk.Duration.seconds(30),
    });

    // Lambda function for saves endpoints
    const savesFunction = new lambda.Function(this, 'SavesFunction', {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('../lambda/gruesome-api/target/lambda/saves'),
      environment: {
        TABLE_NAME: props.table.tableName,
        SAVES_BUCKET: props.savesBucket.bucketName,
      },
      timeout: cdk.Duration.seconds(30),
    });

    // Grant permissions
    props.table.grantReadWriteData(authFunction);
    props.table.grantReadWriteData(gamesFunction);
    props.table.grantReadWriteData(savesFunction);
    props.savesBucket.grantReadWrite(savesFunction);

    // HTTP API Gateway
    const httpApi = new apigatewayv2.HttpApi(this, 'GruesomeApi', {
      apiName: 'gruesome-api',
      corsPreflight: {
        allowOrigins: ['https://gruesome.skeptomai.com'],
        allowMethods: [
          apigatewayv2.CorsHttpMethod.GET,
          apigatewayv2.CorsHttpMethod.POST,
          apigatewayv2.CorsHttpMethod.DELETE,
        ],
        allowHeaders: ['Authorization', 'Content-Type'],
      },
    });

    // Add routes
    httpApi.addRoutes({
      path: '/api/auth/login',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: new apigatewayv2_integrations.HttpLambdaIntegration(
        'AuthIntegration',
        authFunction
      ),
    });

    httpApi.addRoutes({
      path: '/api/games',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: new apigatewayv2_integrations.HttpLambdaIntegration(
        'GamesIntegration',
        gamesFunction
      ),
    });

    httpApi.addRoutes({
      path: '/api/games/{gameId}/save',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: new apigatewayv2_integrations.HttpLambdaIntegration(
        'SavesIntegration',
        savesFunction
      ),
    });

    // Custom domain for API
    const apiDomain = new apigatewayv2.DomainName(this, 'ApiDomain', {
      domainName: 'api.gruesome.skeptomai.com',
      certificate: props.certificate,
    });

    new apigatewayv2.ApiMapping(this, 'ApiMapping', {
      api: httpApi,
      domainName: apiDomain,
    });

    // Route 53 record for API
    new route53.ARecord(this, 'ApiAliasRecord', {
      zone: props.hostedZone,
      recordName: 'api.gruesome',
      target: route53.RecordTarget.fromAlias(
        new route53_targets.ApiGatewayv2DomainProperties(
          apiDomain.regionalDomainName,
          apiDomain.regionalHostedZoneId
        )
      ),
    });

    // Outputs
    new cdk.CfnOutput(this, 'ApiUrl', {
      value: `https://api.gruesome.skeptomai.com`,
    });
  }
}
```

#### 2.5: Frontend Stack (`lib/frontend-stack.ts`)

```typescript
import * as cdk from 'aws-cdk-lib';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as cloudfront_origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as route53 from 'aws-cdk-lib/aws-route53';
import * as route53_targets from 'aws-cdk-lib/aws-route53-targets';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import { Construct } from 'constructs';

interface FrontendStackProps extends cdk.StackProps {
  hostedZone: route53.IHostedZone;
  certificate: acm.Certificate;
}

export class FrontendStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    super(scope, id, props);

    // S3 bucket for static website
    const websiteBucket = new s3.Bucket(this, 'WebsiteBucket', {
      bucketName: 'gruesome-frontend',
      publicReadAccess: false, // CloudFront will access via OAI
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // CloudFront Origin Access Identity
    const oai = new cloudfront.OriginAccessIdentity(this, 'OAI', {
      comment: 'OAI for gruesome-frontend',
    });

    websiteBucket.grantRead(oai);

    // CloudFront distribution
    const distribution = new cloudfront.Distribution(this, 'Distribution', {
      defaultBehavior: {
        origin: new cloudfront_origins.S3Origin(websiteBucket, {
          originAccessIdentity: oai,
        }),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
      },
      domainNames: ['gruesome.skeptomai.com'],
      certificate: props.certificate,
      defaultRootObject: 'index.html',
      errorResponses: [
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.seconds(0),
        },
      ],
    });

    // Route 53 record for frontend
    new route53.ARecord(this, 'FrontendAliasRecord', {
      zone: props.hostedZone,
      recordName: 'gruesome',
      target: route53.RecordTarget.fromAlias(
        new route53_targets.CloudFrontTarget(distribution)
      ),
    });

    // Deploy WASM bundle to S3
    new s3deploy.BucketDeployment(this, 'DeployWebsite', {
      sources: [s3deploy.Source.asset('../wasm/pkg')],
      destinationBucket: websiteBucket,
      distribution,
      distributionPaths: ['/*'],
    });

    // Outputs
    new cdk.CfnOutput(this, 'WebsiteUrl', {
      value: `https://gruesome.skeptomai.com`,
    });
    new cdk.CfnOutput(this, 'DistributionId', {
      value: distribution.distributionId,
    });
    new cdk.CfnOutput(this, 'BucketName', {
      value: websiteBucket.bucketName,
    });
  }
}
```

#### 2.6: Main App (`bin/gruesome-platform.ts`)

```typescript
#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { DnsStack } from '../lib/dns-stack';
import { DataStack } from '../lib/data-stack';
import { AuthStack } from '../lib/auth-stack';
import { BackendStack } from '../lib/backend-stack';
import { FrontendStack } from '../lib/frontend-stack';

const app = new cdk.App();

// DNS stack (us-east-1 for CloudFront certificate requirement)
const dnsStack = new DnsStack(app, 'GruesomeDnsStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1', // Required for CloudFront certificates
  },
  crossRegionReferences: true,
});

// Data stack (can be any region)
const dataStack = new DataStack(app, 'GruesomeDataStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION || 'us-west-2',
  },
});

// Auth stack
const authStack = new AuthStack(app, 'GruesomeAuthStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION || 'us-west-2',
  },
});

// Backend stack (depends on data and auth)
const backendStack = new BackendStack(app, 'GruesomeBackendStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION || 'us-west-2',
  },
  table: dataStack.table,
  savesBucket: dataStack.savesBucket,
  userPool: authStack.userPool,
  hostedZone: dnsStack.hostedZone,
  certificate: dnsStack.certificate,
});

// Frontend stack (must be us-east-1 for CloudFront)
const frontendStack = new FrontendStack(app, 'GruesomeFrontendStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1', // Required for CloudFront
  },
  hostedZone: dnsStack.hostedZone,
  certificate: dnsStack.certificate,
  crossRegionReferences: true,
});

// Add dependencies
backendStack.addDependency(dataStack);
backendStack.addDependency(authStack);
backendStack.addDependency(dnsStack);
frontendStack.addDependency(dnsStack);
```

### Step 3: Build Rust Lambda Functions

Create minimal Rust Lambda functions:

```bash
cd infrastructure
mkdir -p lambda/gruesome-api
cd lambda/gruesome-api

# Initialize Rust workspace
cat > Cargo.toml << 'EOF'
[workspace]
members = ["auth", "games", "saves"]
EOF

# Create auth function
cargo new --bin auth
cd auth

# Add dependencies to auth/Cargo.toml
cat >> Cargo.toml << 'EOF'
[dependencies]
lambda_runtime = "0.8"
lambda_http = "0.8"
tokio = { version = "1", features = ["macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
aws-sdk-dynamodb = "1.0"
aws-config = "1.0"
EOF

# Minimal auth/src/main.rs
cat > src/main.rs << 'EOF'
use lambda_http::{run, service_fn, Body, Error, Request, Response};

async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"message": "Auth endpoint placeholder"}"#.into())
        .unwrap();
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
EOF

# Build for Lambda
cd ..
cargo lambda build --release --arm64
```

Repeat for `games` and `saves` functions (or create symlinks initially).

### Step 4: Deploy Infrastructure

```bash
cd infrastructure

# Preview changes
cdk diff

# Deploy all stacks
cdk deploy --all

# Or deploy individually
cdk deploy GruesomeDnsStack
cdk deploy GruesomeDataStack
cdk deploy GruesomeAuthStack
cdk deploy GruesomeBackendStack
cdk deploy GruesomeFrontendStack
```

CDK will:
1. Create ACM certificate (DNS validation automatic)
2. Create DynamoDB table with GSI
3. Create S3 buckets (frontend + saves)
4. Create Cognito User Pool
5. Deploy Lambda functions
6. Create API Gateway with custom domain
7. Create CloudFront distribution
8. Create Route 53 A records
9. Deploy WASM frontend to S3

**All automated - zero manual AWS console clicking!**

### Step 5: Verify Deployment

```bash
# Check deployed resources
cdk ls

# Get outputs
aws cloudformation describe-stacks --stack-name GruesomeFrontendStack \
  --query 'Stacks[0].Outputs'

# Test frontend
curl https://gruesome.skeptomai.com

# Test API
curl https://api.gruesome.skeptomai.com/api/auth/login
```

### Step 6: Update GitHub Pages (Optional Redirect)

Add redirect to old GitHub Pages site:

```html
<!-- index.html in GitHub Pages -->
<!DOCTYPE html>
<html>
<head>
  <meta http-equiv="refresh" content="0; url=https://gruesome.skeptomai.com">
  <title>Redirecting...</title>
</head>
<body>
  <p>This site has moved to <a href="https://gruesome.skeptomai.com">gruesome.skeptomai.com</a></p>
</body>
</html>
```

---

## CI/CD Integration (Future)

Add GitHub Actions workflow to auto-deploy on push:

```yaml
# .github/workflows/deploy.yml
name: Deploy to AWS
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v2
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-west-2

      - name: Install CDK
        run: npm install -g aws-cdk

      - name: Build Rust functions
        run: |
          cd infrastructure/lambda/gruesome-api
          cargo lambda build --release --arm64

      - name: Deploy infrastructure
        run: |
          cd infrastructure
          npm ci
          cdk deploy --all --require-approval never
```

---

## Cost Estimate for Infrastructure

**CDK Deployment Itself**: Free (just CloudFormation)

**Resources Created**:
- Route 53 hosted zone: $0.50/month (existing, no change)
- ACM certificate: FREE
- DynamoDB: Free tier (permanent)
- S3 (frontend): ~$0.50/month
- S3 (saves): Pay per use
- CloudFront: Free tier (1TB/month)
- Lambda: Free tier (1M requests/month)
- API Gateway: Free tier (1M requests/month first year)
- Cognito: Free tier (50k MAU)

**Total**: ~$1-2/month for minimal usage

---

## Rollback / Cleanup

```bash
# Destroy all infrastructure (BE CAREFUL!)
cdk destroy --all

# Destroy specific stack
cdk destroy GruesomeFrontendStack

# Note: Stacks with RETAIN policy (DynamoDB, S3) won't delete data
```

---

## Next Steps After Deployment

1. **Configure GitHub OAuth App**
   - Create OAuth app at github.com/settings/developers
   - Set callback URL: `https://gruesome.skeptomai.com/auth/callback`
   - Update `auth-stack.ts` with client ID/secret

2. **Implement Lambda Functions**
   - Replace placeholder functions with real logic
   - Add DynamoDB queries
   - Add S3 save/load operations
   - Add Cognito JWT validation

3. **Update WASM Frontend**
   - Change API endpoint to `https://api.gruesome.skeptomai.com`
   - Add login UI
   - Implement JWT token storage
   - Add save browser UI

4. **Test End-to-End**
   - Register user via Cognito
   - Login via frontend
   - Save game to S3
   - Load game from S3

5. **Monitor and Optimize**
   - Set up CloudWatch dashboards
   - Monitor Lambda cold starts
   - Optimize DynamoDB queries
   - Add error tracking (Sentry, etc.)

---

## Advantages of CDK Approach

1. **Zero Manual Configuration**: Everything defined in code
2. **Repeatable**: Destroy and recreate identical infrastructure
3. **Version Controlled**: Infrastructure changes tracked in git
4. **Type Safe**: TypeScript catches errors before deployment
5. **Preview Changes**: `cdk diff` shows exactly what will change
6. **Rollback**: Easy to revert to previous stack version
7. **Documentation**: Code IS the documentation
8. **Multi-Environment**: Easy to create dev/staging/prod environments
9. **Cross-Stack References**: CDK handles dependencies automatically
10. **Best Practices**: CDK applies AWS security best practices by default

---

## Troubleshooting

**Issue**: Certificate validation stuck

**Solution**: Verify DNS is pointing to Route 53, wait up to 30 minutes for DNS propagation

**Issue**: Lambda function not found

**Solution**: Build Rust functions first: `cargo lambda build --release --arm64`

**Issue**: CloudFront distribution creation slow

**Solution**: Normal - CloudFront takes 15-30 minutes to deploy globally

**Issue**: API Gateway 403 errors

**Solution**: Check CORS configuration and Cognito authorizer setup

**Issue**: S3 bucket name already taken

**Solution**: S3 bucket names are globally unique - change `bucketName` in stacks

---

## Summary

This plan provides **complete infrastructure as code** for the multi-user platform using AWS CDK.

**Single command deployment**: `cdk deploy --all`

**Zero manual AWS console configuration** required!

All infrastructure is version controlled, repeatable, and documented in TypeScript code.
