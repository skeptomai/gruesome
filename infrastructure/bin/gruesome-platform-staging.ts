#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { GruesomeDnsStack } from '../lib/dns-stack';
import { GruesomeDataStack } from '../lib/data-stack';
import { GruesomeAuthStack } from '../lib/auth-stack';
import { GruesomeBackendStack } from '../lib/backend-stack';
import { GruesomeFrontendStack } from '../lib/frontend-stack';

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

// DNS Stack (staging subdomain)
const dnsStackStaging = new GruesomeDnsStack(app, 'GruesomeDnsStackStaging', {
  env: frontendStagingEnv,
  crossRegionReferences: true,
  domainName: 'skeptomai.com',
  subdomainPrefix: 'staging.gruesome', // staging.gruesome.skeptomai.com
  apiSubdomainPrefix: 'api-staging.gruesome', // api-staging.gruesome.skeptomai.com
});

// Data Stack (staging - separate DynamoDB and S3)
const dataStackStaging = new GruesomeDataStack(app, 'GruesomeDataStackStaging', {
  env: stagingEnv,
  stackPrefix: 'staging',
});

// Auth Stack (staging - separate Cognito)
const authStackStaging = new GruesomeAuthStack(app, 'GruesomeAuthStackStaging', {
  env: stagingEnv,
  stackPrefix: 'staging',
});

// Backend Stack (staging API)
const backendStackStaging = new GruesomeBackendStack(app, 'GruesomeBackendStackStaging', {
  env: stagingEnv,
  crossRegionReferences: true,
  certificate: dnsStackStaging.certificate,
  hostedZone: dnsStackStaging.hostedZone,
  table: dataStackStaging.table,
  savesBucket: dataStackStaging.savesBucket,
  gamesBucket: dataStackStaging.gamesBucket,
  userPool: authStackStaging.userPool,
  userPoolClient: authStackStaging.userPoolClient,
  stackPrefix: 'staging',
  apiDomainName: 'api-staging.gruesome.skeptomai.com',
});

// Frontend Stack (staging website)
const frontendStackStaging = new GruesomeFrontendStack(app, 'GruesomeFrontendStackStaging', {
  env: frontendStagingEnv,
  crossRegionReferences: true,
  certificate: dnsStackStaging.certificate,
  hostedZone: dnsStackStaging.hostedZone,
  stackPrefix: 'staging',
  domainName: 'staging.gruesome.skeptomai.com',
});

// Add dependencies
frontendStackStaging.addDependency(dnsStackStaging);
backendStackStaging.addDependency(dnsStackStaging);
