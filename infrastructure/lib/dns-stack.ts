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
