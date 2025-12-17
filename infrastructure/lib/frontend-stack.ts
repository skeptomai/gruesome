import * as cdk from 'aws-cdk-lib';
import * as s3 from 'aws-cdk-lib/aws-s3';
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
      publicReadAccess: false,
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
