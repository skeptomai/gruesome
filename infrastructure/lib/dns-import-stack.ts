import * as cdk from 'aws-cdk-lib';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as route53 from 'aws-cdk-lib/aws-route53';
import { Construct } from 'constructs';

interface DnsImportStackProps extends cdk.StackProps {
  certificateArn: string;
  domainName: string;
}

/**
 * DnsImportStack - Imports existing DNS resources (certificate and hosted zone)
 *
 * This stack imports the ACM certificate and Route53 hosted zone by reference
 * rather than creating or managing them. This approach:
 *
 * 1. Avoids CloudFormation cross-region export limitations (exports are immutable)
 * 2. Allows the certificate to be shared across multiple stacks without crossRegionReferences
 * 3. Follows AWS best practices for hardcoding static resource ARNs
 *
 * The certificate is not managed by CloudFormation (orphaned resource).
 * It was created by the old GruesomeDnsStack and retained when that stack was deleted.
 * This is the ideal state for long-lived shared resources.
 */
export class DnsImportStack extends cdk.Stack {
  public readonly certificate: acm.ICertificate;
  public readonly hostedZone: route53.IHostedZone;

  constructor(scope: Construct, id: string, props: DnsImportStackProps) {
    super(scope, id, props);

    // Import existing certificate by ARN (not managed by this stack)
    this.certificate = acm.Certificate.fromCertificateArn(
      this,
      'ImportedCertificate',
      props.certificateArn
    );

    // Import existing hosted zone by domain lookup
    this.hostedZone = route53.HostedZone.fromLookup(this, 'ImportedHostedZone', {
      domainName: props.domainName,
    });
  }
}
