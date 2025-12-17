import * as cdk from 'aws-cdk-lib';
import * as dynamodb from 'aws-cdk-lib/aws-dynamodb';
import * as s3 from 'aws-cdk-lib/aws-s3';
import { Construct } from 'constructs';

export class DataStack extends cdk.Stack {
  public readonly table: dynamodb.Table;
  public readonly savesBucket: s3.Bucket;
  public readonly gamesBucket: s3.Bucket;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // DynamoDB table with single-table design
    this.table = new dynamodb.Table(this, 'GruesomePlatformTable', {
      tableName: 'gruesome-platform',
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
      bucketName: 'gruesome-saves',
      versioned: true,
      lifecycleRules: [
        {
          noncurrentVersionExpiration: cdk.Duration.days(90),
        },
      ],
      cors: [
        {
          allowedOrigins: ['https://gruesome.skeptomai.com'],
          allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT, s3.HttpMethods.DELETE],
          allowedHeaders: ['*'],
        },
      ],
      removalPolicy: cdk.RemovalPolicy.RETAIN,
    });

    // S3 bucket for game files (read-only, holds Z-Machine files)
    this.gamesBucket = new s3.Bucket(this, 'GruesomeGamesBucket', {
      bucketName: 'gruesome-games',
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
