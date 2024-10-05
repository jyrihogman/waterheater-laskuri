import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import type { Tags } from "./index";
import { createLambdaAlarms } from "./alarms";

export function provisionMonthlyPricingWorker(
  eventRole: aws.iam.Role,
  alarmTopic: aws.sns.Topic,
  config: pulumi.Config,
  commonTags: Tags,
) {
  const entsoeApiToken = config.require("entsoeApiToken");

  const queue = new aws.sqs.Queue("getMonthlyPricingEventQueue", {
    tags: commonTags,
    name: "GetMonthlyPricingEventQueue",
    visibilityTimeoutSeconds: 600,
    delaySeconds: 60,
  });

  const dlq = new aws.sqs.Queue("getMonthlyPricingEventDLQ", {
    name: "GetMonthlyPricingEventDLQ",
    messageRetentionSeconds: 1209600,
    visibilityTimeoutSeconds: 120,
  });

  new aws.sqs.RedrivePolicy("monthlyPricingQueueRedrivePolicy", {
    queueUrl: queue.id,
    redrivePolicy: pulumi.jsonStringify({
      deadLetterTargetArn: dlq.arn,
      maxReceiveCount: 5,
    }),
  });

  new aws.scheduler.Schedule("dailyMonthlyPricingMessageSchedule", {
    name: "GetMonthlyElectricityPricingSchedule",
    groupName: "default",
    flexibleTimeWindow: {
      mode: "OFF",
    },
    scheduleExpression: "cron(0 13 * * ? *)",
    target: {
      retryPolicy: {
        maximumRetryAttempts: 5,
        maximumEventAgeInSeconds: 5000,
      },
      arn: queue.arn,
      roleArn: eventRole.arn,
      deadLetterConfig: {
        arn: dlq.arn,
      },
    },
  });

  const dynamoTable = new aws.dynamodb.Table("monthlyElectricityPricingData", {
    name: "electricity_pricing_monthly",
    attributes: [
      { name: "country", type: "S" },
      { name: "date", type: "S" },
    ],
    hashKey: "country",
    rangeKey: "date",
    billingMode: "PROVISIONED",
    writeCapacity: 1,
    readCapacity: 1,
    tags: {
      ...commonTags,
    },
  });

  const worker = new aws.lambda.Function(
    "waterheater-calc-monthly-pricing-worker",
    {
      name: "wh-electricity-monthly-pricing-worker",
      tags: commonTags,
      code: new pulumi.asset.AssetArchive({
        bootstrap: new pulumi.asset.FileAsset(
          "../pricing-worker/lambda-handler.zip",
        ),
      }),
      handler: "bootstrap",
      runtime: aws.lambda.Runtime.CustomAL2023,
      environment: {
        variables: {
          ENTSOE_TOKEN: entsoeApiToken,
        },
      },
      role: createWorkerRole(dynamoTable, queue, commonTags).arn,
      timeout: 60,
    },
  );

  createLambdaAlarms(worker, "PricingWorker", alarmTopic);

  new aws.lambda.Permission("sqsInvokeMonthlyPricingWorker", {
    action: "lambda:InvokeFunction",
    function: worker.name,
    principal: "sqs.amazonaws.com",
    sourceArn: queue.arn,
  });

  new aws.lambda.EventSourceMapping("sqsMonthlyPricingWorkerLambdaTrigger", {
    eventSourceArn: queue.arn,
    functionName: worker.arn,
    batchSize: 1, // Process one message at a time
  });
}

function createWorkerRole(
  dynamoTable: aws.dynamodb.Table,
  queue: aws.sqs.Queue,
  commonTags: Tags,
) {
  const lambdaDynamoDbPolicy = new aws.iam.Policy(
    "lambda-monthly-worker-policy",
    {
      tags: commonTags,
      description: "IAM policy for Lambda to have PutItem access to DynamoDB",
      policy: {
        Version: "2012-10-17",
        Statement: [
          {
            Action: ["dynamodb:PutItem"],
            Effect: "Allow",
            Resource: [dynamoTable.arn],
          },
        ],
      },
    },
  );

  const lambdaSQSPolicy = new aws.iam.Policy("monthly-worker-sqs-policy", {
    tags: commonTags,
    description: "IAM policy for Lambda to have PutItem access to DynamoDB",
    policy: queue.arn.apply((arn) =>
      JSON.stringify({
        Version: "2012-10-17",
        Statement: [
          {
            Effect: "Allow",
            Action: [
              "sqs:ReceiveMessage",
              "sqs:DeleteMessage",
              "SQS:GetQueueAttributes",
            ],
            Resource: arn,
          },
        ],
      }),
    ),
  });

  const role = new aws.iam.Role(
    "waterheater-calc-monthly-pricing-worker-role",
    {
      tags: commonTags,
      assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
        Service: "lambda.amazonaws.com",
      }),
    },
  );

  new aws.iam.RolePolicyAttachment(
    "monthly-pricing-worker-execute-policy-attachment",
    {
      role: role.name,
      policyArn: aws.iam.ManagedPolicy.AWSLambdaExecute,
    },
  );

  new aws.iam.RolePolicyAttachment(
    "monthly-pricing-worker-dynamo-execute-policy",
    {
      role: role.name,
      policyArn: lambdaDynamoDbPolicy.arn,
    },
  );

  new aws.iam.RolePolicyAttachment("monthly-pricing-worker-sqs-attachment", {
    role: role.name,
    policyArn: lambdaSQSPolicy.arn,
  });

  return role;
}
