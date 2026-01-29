#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { DataStack } from '../lib/data-stack';
import { AuthStack } from '../lib/auth-stack';
import { BackendStack } from '../lib/backend-stack';
import { FrontendStack } from '../lib/frontend-stack';
import { DnsImportStack } from '../lib/dns-import-stack';

const app = new cdk.App();

// Environment configuration for staging
const stagingEnv = {
  account: process.env.CDK_DEFAULT_ACCOUNT,
  region: 'us-west-1', // Backend in us-west-1
};

const frontendStagingEnv = {
  account: process.env.CDK_DEFAULT_ACCOUNT,
  region: 'us-east-1', // CloudFront requires us-east-1
};

// Import existing DNS resources from production
// The wildcard cert *.gruesome.skeptomai.com covers both production and staging
//
// CERTIFICATE MANAGEMENT:
// The certificate ARN is hardcoded per AWS best practices for static resources.
// Certificate ARNs never change (renewal doesn't change the ARN).
// The certificate is NOT managed by CloudFormation (orphaned resource).
// It was created by the old GruesomeDnsStack and retained when that stack was deleted.
// This is the ideal state for long-lived shared resources.
//
// ARCHITECTURE DECISION:
// No crossRegionReferences flag - avoids CloudFormation export limitations.
// Exports are immutable once in use, causing deployment failures.
// Hardcoding the ARN allows clean infrastructure updates without workarounds.
const dnsImportStack = new DnsImportStack(app, 'GruesomeDnsImportStackStaging', {
  env: frontendStagingEnv,
  certificateArn: 'arn:aws:acm:us-east-1:349145659387:certificate/33ae9627-b894-4edc-a480-201bc6e8b529',
  domainName: 'skeptomai.com',
});

// Data Stack (staging - separate DynamoDB and S3)
const dataStackStaging = new DataStack(app, 'GruesomeDataStackStaging', {
  env: stagingEnv,
  tableName: 'gruesome-platform-staging',
  savesBucketName: 'gruesome-saves-staging',
  gamesBucketName: 'gruesome-games-staging',
  frontendOrigin: 'https://staging.gruesome.skeptomai.com',
});

// Auth Stack (staging - separate Cognito)
const authStackStaging = new AuthStack(app, 'GruesomeAuthStackStaging', {
  env: stagingEnv,
});

// Backend Stack (staging API with custom domain)
const backendStackStaging = new BackendStack(app, 'GruesomeBackendStackStaging', {
  env: stagingEnv,
  table: dataStackStaging.table,
  savesBucket: dataStackStaging.savesBucket,
  gamesBucket: dataStackStaging.gamesBucket,
  userPool: authStackStaging.userPool,
  userPoolClient: authStackStaging.userPoolClient,
  hostedZone: dnsImportStack.hostedZone,
  certificate: dnsImportStack.certificate,
  apiDomainName: 'api-staging.gruesome.skeptomai.com',
  apiSubdomain: 'api-staging.gruesome',
  frontendUrl: 'https://staging.gruesome.skeptomai.com',
  environmentName: 'Staging',
});

// Frontend Stack (staging website with custom domain)
const frontendStackStaging = new FrontendStack(app, 'GruesomeFrontendStackStaging', {
  env: frontendStagingEnv,
  bucketName: 'gruesome-frontend-staging',
  hostedZone: dnsImportStack.hostedZone,
  certificate: dnsImportStack.certificate,
  domainName: 'staging.gruesome.skeptomai.com',
  subdomain: 'staging.gruesome',
});

// Dependencies
backendStackStaging.addDependency(dataStackStaging);
backendStackStaging.addDependency(authStackStaging);
backendStackStaging.addDependency(dnsImportStack);
frontendStackStaging.addDependency(dnsImportStack);
