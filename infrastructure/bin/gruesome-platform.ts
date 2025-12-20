#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { DnsImportStack } from '../lib/dns-import-stack';
import { DataStack } from '../lib/data-stack';
import { AuthStack } from '../lib/auth-stack';
import { BackendStack } from '../lib/backend-stack';
import { FrontendStack } from '../lib/frontend-stack';

const app = new cdk.App();

// Import existing DNS resources (certificate and hosted zone)
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
const dnsImportStack = new DnsImportStack(app, 'GruesomeDnsImportStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',  // Certificate must be in us-east-1 for CloudFront
  },
  certificateArn: 'arn:aws:acm:us-east-1:349145659387:certificate/33ae9627-b894-4edc-a480-201bc6e8b529',
  domainName: 'skeptomai.com',
});

// Data stack (us-west-1)
const dataStack = new DataStack(app, 'GruesomeDataStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-west-1',
  },
});

// Auth stack (us-west-1)
const authStack = new AuthStack(app, 'GruesomeAuthStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-west-1',
  },
});

// Backend stack (us-west-1)
const backendStack = new BackendStack(app, 'GruesomeBackendStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-west-1',
  },
  table: dataStack.table,
  savesBucket: dataStack.savesBucket,
  gamesBucket: dataStack.gamesBucket,
  userPool: authStack.userPool,
  userPoolClient: authStack.userPoolClient,
  hostedZone: dnsImportStack.hostedZone,
  certificate: dnsImportStack.certificate,
});

// Frontend stack (us-east-1 for CloudFront)
const frontendStack = new FrontendStack(app, 'GruesomeFrontendStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  hostedZone: dnsImportStack.hostedZone,
  certificate: dnsImportStack.certificate,
});

// Dependencies
backendStack.addDependency(dataStack);
backendStack.addDependency(authStack);
backendStack.addDependency(dnsImportStack);
frontendStack.addDependency(dnsImportStack);
