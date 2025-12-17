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
    region: 'us-east-1',
  },
  crossRegionReferences: true,
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
  crossRegionReferences: true,
  table: dataStack.table,
  savesBucket: dataStack.savesBucket,
  gamesBucket: dataStack.gamesBucket,
  userPool: authStack.userPool,
  userPoolClient: authStack.userPoolClient,
  hostedZone: dnsStack.hostedZone,
  certificate: dnsStack.certificate,
});

// Frontend stack (us-east-1 for CloudFront)
const frontendStack = new FrontendStack(app, 'GruesomeFrontendStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  hostedZone: dnsStack.hostedZone,
  certificate: dnsStack.certificate,
  crossRegionReferences: true,
});

// Dependencies
backendStack.addDependency(dataStack);
backendStack.addDependency(authStack);
backendStack.addDependency(dnsStack);
frontendStack.addDependency(dnsStack);
