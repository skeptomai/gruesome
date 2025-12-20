import * as cdk from 'aws-cdk-lib';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';
import { Construct } from 'constructs';

interface DataStackProps extends cdk.StackProps {
  tableName?: string;         // Optional custom table name for staging
  savesBucketName?: string;   // Optional custom saves bucket name for staging
  gamesBucketName?: string;   // Optional custom games bucket name for staging
  frontendOrigin?: string;    // Frontend origin for CORS (e.g., 'https://staging.gruesome.skeptomai.com')
}

export class DataStack extends cdk.Stack {
  public readonly table: dynamodb.Table;
  public readonly savesBucket: s3.Bucket;
  public readonly gamesBucket: s3.Bucket;

  constructor(scope: Construct, id: string, props?: DataStackProps) {
    super(scope, id, props);

    // DynamoDB table with single-table design
    this.table = new dynamodb.Table(this, 'GruesomePlatformTable', {
      tableName: props?.tableName || 'gruesome-platform',
      partitionKey: {
        name: 'PK',
        type: dynamodb.AttributeType.STRING,
      },
      sortKey: {
        name: 'SK',
        type: dynamodb.AttributeType.STRING,
      },
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      pointInTimeRecovery: true,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
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

    // Global Secondary Index for entity type queries (games, saves, etc.)
    this.table.addGlobalSecondaryIndex({
      indexName: 'entity-type-index',
      partitionKey: {
        name: 'entity_type',
        type: dynamodb.AttributeType.STRING,
      },
      sortKey: {
        name: 'created_at',
        type: dynamodb.AttributeType.NUMBER,
      },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // S3 bucket for save files
    this.savesBucket = new s3.Bucket(this, 'GruesomeSavesBucket', {
      bucketName: props?.savesBucketName || 'gruesome-saves',
      versioned: true,
      lifecycleRules: [
        {
          noncurrentVersionExpiration: cdk.Duration.days(90),
        },
      ],
      cors: [
        {
          // Avoid duplicate origins (CloudFormation validation error)
          // Staging: allow both staging and production origins
          // Production: allow only production origin
          allowedOrigins: props?.frontendOrigin && props.frontendOrigin !== 'https://gruesome.skeptomai.com'
            ? [props.frontendOrigin, 'https://gruesome.skeptomai.com']  // Staging: both origins
            : ['https://gruesome.skeptomai.com'],  // Production: single origin
          allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT, s3.HttpMethods.DELETE],
          allowedHeaders: ['*'],
          maxAge: 3000,
        },
      ],
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // S3 bucket for game files (read-only, holds Z-Machine files)
    this.gamesBucket = new s3.Bucket(this, 'GruesomeGamesBucket', {
      bucketName: props?.gamesBucketName || 'gruesome-games',
      publicReadAccess: false,
      versioned: false,
      encryption: s3.BucketEncryption.S3_MANAGED,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // Outputs
    new cdk.CfnOutput(this, 'TableName', {
      value: this.table.tableName,
    });
    new cdk.CfnOutput(this, 'SavesBucketName', {
      value: this.savesBucket.bucketName,
    });
    new cdk.CfnOutput(this, 'GamesBucketName', {
      value: this.gamesBucket.bucketName,
    });
  }
}
