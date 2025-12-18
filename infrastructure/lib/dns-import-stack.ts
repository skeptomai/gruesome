import * as cdk from 'aws-cdk-lib';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as route53 from 'aws-cdk-lib/aws-route53';
import { Construct } from 'constructs';

interface DnsImportStackProps extends cdk.StackProps {
  certificateArn: string;
  domainName: string;
}

export class DnsImportStack extends cdk.Stack {
  public readonly certificate: acm.ICertificate;
  public readonly hostedZone: route53.IHostedZone;

  constructor(scope: Construct, id: string, props: DnsImportStackProps) {
    super(scope, id, props);

    // Import existing certificate from production
    this.certificate = acm.Certificate.fromCertificateArn(
      this,
      'ImportedCertificate',
      props.certificateArn
    );

    // Import existing hosted zone from production
    this.hostedZone = route53.HostedZone.fromLookup(this, 'ImportedHostedZone', {
      domainName: props.domainName,
    });
  }
}
