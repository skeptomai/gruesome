import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigatewayv2 from 'aws-cdk-lib/aws-apigatewayv2';
import * as apigatewayv2_integrations from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as cloudfront_origins from 'aws-cdk-lib/aws-cloudfront-origins';
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
  gamesBucket: s3.Bucket;
  userPool: cognito.UserPool;
  userPoolClient: cognito.UserPoolClient;
  hostedZone: route53.IHostedZone;
  certificate: acm.Certificate;
}

export class BackendStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: BackendStackProps) {
    super(scope, id, props);

    // Lambda function for auth endpoints
    const authFunction = new lambda.Function(this, 'AuthFunction', {
      runtime: lambda.Runtime.PROVIDED_AL2023,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('./lambda/gruesome-api/target/lambda/auth'),
      environment: {
        TABLE_NAME: props.table.tableName,
        USER_POOL_ID: props.userPool.userPoolId,
        USER_POOL_CLIENT_ID: props.userPoolClient.userPoolClientId,
      },
      timeout: cdk.Duration.seconds(30),
      architecture: lambda.Architecture.ARM_64,
    });

    // Grant permissions
    props.table.grantReadWriteData(authFunction);

    // Grant Cognito admin permissions for auto-confirming users
    props.userPool.grant(authFunction,
      'cognito-idp:AdminConfirmSignUp',
      'cognito-idp:AdminGetUser'
    );

    // Lambda function for game endpoints
    const gameFunction = new lambda.Function(this, 'GameFunction', {
      runtime: lambda.Runtime.PROVIDED_AL2023,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset('./lambda/gruesome-api/target/lambda/game'),
      environment: {
        TABLE_NAME: props.table.tableName,
        GAMES_BUCKET: props.gamesBucket.bucketName,
        SAVES_BUCKET: props.savesBucket.bucketName,
        USER_POOL_ID: props.userPool.userPoolId,
      },
      timeout: cdk.Duration.seconds(30),
      memorySize: 512,
      architecture: lambda.Architecture.ARM_64,
    });

    // Grant permissions for game Lambda
    props.table.grantReadWriteData(gameFunction);
    props.gamesBucket.grantRead(gameFunction);
    props.savesBucket.grantReadWrite(gameFunction);

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
    const authIntegration = new apigatewayv2_integrations.HttpLambdaIntegration(
      'AuthIntegration',
      authFunction
    );

    // Signup endpoint
    httpApi.addRoutes({
      path: '/api/auth/signup',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: authIntegration,
    });

    // Login endpoint
    httpApi.addRoutes({
      path: '/api/auth/login',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: authIntegration,
    });

    // Refresh token endpoint
    httpApi.addRoutes({
      path: '/api/auth/refresh',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: authIntegration,
    });

    // Forgot password endpoint
    httpApi.addRoutes({
      path: '/api/auth/forgot-password',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: authIntegration,
    });

    // Confirm forgot password endpoint
    httpApi.addRoutes({
      path: '/api/auth/confirm-forgot-password',
      methods: [apigatewayv2.HttpMethod.POST],
      integration: authIntegration,
    });

    // Get user profile endpoint
    httpApi.addRoutes({
      path: '/api/auth/me',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: authIntegration,
    });

    // Health check endpoint
    httpApi.addRoutes({
      path: '/health',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: authIntegration,
    });

    // Game endpoints
    const gameIntegration = new apigatewayv2_integrations.HttpLambdaIntegration(
      'GameIntegration',
      gameFunction
    );

    // List all games
    httpApi.addRoutes({
      path: '/api/games',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: gameIntegration,
    });

    // Get game metadata
    httpApi.addRoutes({
      path: '/api/games/{game_id}',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: gameIntegration,
    });

    // Get game file download URL
    httpApi.addRoutes({
      path: '/api/games/{game_id}/file',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: gameIntegration,
    });

    // List all saves for user
    httpApi.addRoutes({
      path: '/api/saves',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: gameIntegration,
    });

    // List saves for specific game
    httpApi.addRoutes({
      path: '/api/saves/{game_id}',
      methods: [apigatewayv2.HttpMethod.GET],
      integration: gameIntegration,
    });

    // Get/create/delete specific save
    httpApi.addRoutes({
      path: '/api/saves/{game_id}/{save_name}',
      methods: [
        apigatewayv2.HttpMethod.GET,
        apigatewayv2.HttpMethod.POST,
        apigatewayv2.HttpMethod.DELETE,
      ],
      integration: gameIntegration,
    });

    // Extract API Gateway URL (format: https://{api-id}.execute-api.{region}.amazonaws.com)
    const apiUrl = httpApi.apiEndpoint;

    // CloudFront distribution in front of API Gateway
    // Custom cache policy that forwards Authorization header (required for authenticated API requests)
    const apiCachePolicy = new cloudfront.CachePolicy(this, 'ApiCachePolicy', {
      cachePolicyName: 'GruesomeApiCachePolicy',
      comment: 'Cache policy for Gruesome API with Authorization header support',
      defaultTtl: cdk.Duration.seconds(0), // Don't cache API responses
      minTtl: cdk.Duration.seconds(0),
      maxTtl: cdk.Duration.seconds(1),
      headerBehavior: cloudfront.CacheHeaderBehavior.allowList('Authorization', 'Content-Type'),
      queryStringBehavior: cloudfront.CacheQueryStringBehavior.all(),
      cookieBehavior: cloudfront.CacheCookieBehavior.none(),
      enableAcceptEncodingGzip: false,
      enableAcceptEncodingBrotli: false,
    });

    // Response headers policy to forward CORS headers from API Gateway
    const apiResponseHeadersPolicy = new cloudfront.ResponseHeadersPolicy(this, 'ApiResponseHeadersPolicy', {
      responseHeadersPolicyName: 'GruesomeApiResponseHeadersPolicy',
      comment: 'Pass through CORS headers from API Gateway',
      corsBehavior: {
        accessControlAllowOrigins: ['https://gruesome.skeptomai.com'],
        accessControlAllowHeaders: ['*'],
        accessControlAllowMethods: ['GET', 'POST', 'DELETE', 'OPTIONS'],
        accessControlAllowCredentials: false,
        originOverride: false, // Don't override - let API Gateway CORS headers pass through
      },
    });

    const apiDistribution = new cloudfront.Distribution(this, 'ApiDistribution', {
      defaultBehavior: {
        origin: new cloudfront_origins.HttpOrigin(
          cdk.Fn.select(2, cdk.Fn.split('/', apiUrl)), // Extract domain from URL
          {
            protocolPolicy: cloudfront.OriginProtocolPolicy.HTTPS_ONLY,
          }
        ),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
        cachePolicy: apiCachePolicy,
        originRequestPolicy: cloudfront.OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER,
        responseHeadersPolicy: apiResponseHeadersPolicy,
      },
      domainNames: ['api.gruesome.skeptomai.com'],
      certificate: props.certificate,
    });

    // Route 53 record for API pointing to CloudFront
    new route53.ARecord(this, 'ApiAliasRecord', {
      zone: props.hostedZone,
      recordName: 'api.gruesome',
      target: route53.RecordTarget.fromAlias(
        new route53_targets.CloudFrontTarget(apiDistribution)
      ),
    });

    // Outputs
    new cdk.CfnOutput(this, 'ApiUrl', {
      value: `https://api.gruesome.skeptomai.com`,
    });
    new cdk.CfnOutput(this, 'ApiDistributionId', {
      value: apiDistribution.distributionId,
    });
  }
}
