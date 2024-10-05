import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import type { Tags } from "./index";
import { createLambdaAlarms } from "./alarms";

export function provisionWorker(
  queue: aws.sqs.Queue,
  dlq: aws.sqs.Queue,
  eventRole: aws.iam.Role,
  dynamoTable: aws.dynamodb.Table,
  alarmTopic: aws.sns.Topic,
  commonTags: Tags,
) {
  new aws.scheduler.Schedule("dailyMessageSchedule", {
    name: "GetElectricityPricingSchedule",
    groupName: "default",
    flexibleTimeWindow: {
      mode: "OFF",
    },
    scheduleExpression: "cron(0 13 * * ? *)",
    target: {
      retryPolicy: {
        maximumRetryAttempts: 2,
        maximumEventAgeInSeconds: 120,
      },
      arn: queue.arn,
      roleArn: eventRole.arn,
      deadLetterConfig: {
        arn: dlq.arn,
      },
      input: pulumi.jsonStringify({
        retry_attempt: 0,
      }),
    },
  });

  const imageOutput = aws.ecr.getImageOutput({
    repositoryName: "waterheater-calc-worker",
    imageTag: "latest",
  });

  const worker = new aws.lambda.Function("waterheater-calc-pricing-worker", {
    name: "wh-electricity-pricing-worker",
    tags: commonTags,
    imageUri: imageOutput.imageUri,
    packageType: "Image",
    role: createWorkerRole(dynamoTable, queue, commonTags).arn,
    timeout: 60,
  });

  createLambdaAlarms(worker, "Worker", alarmTopic);

  new aws.lambda.Permission("sqsInvokeWorker", {
    action: "lambda:InvokeFunction",
    function: worker.name,
    principal: "sqs.amazonaws.com",
    sourceArn: queue.arn,
  });

  new aws.lambda.EventSourceMapping("sqsWorkerLambdaTrigger", {
    eventSourceArn: queue.arn,
    functionName: worker.arn,
    batchSize: 1, // Process one message at a time
  });

  return worker;
}

function createWorkerRole(
  dynamoTable: aws.dynamodb.Table,
  queue: aws.sqs.Queue,
  commonTags: Tags,
) {
  const lambdaDynamoDbPolicy = new aws.iam.Policy("lambda-dynamodb-policy", {
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
  });

  const lambdaSQSPolicy = new aws.iam.Policy("lambda-sqs-policy", {
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

  const role = new aws.iam.Role("waterheater-calc-worker-role", {
    tags: commonTags,
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
      Service: "lambda.amazonaws.com",
    }),
  });

  new aws.iam.RolePolicyAttachment("lambda-execute-policy-attachment", {
    role: role.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaExecute,
  });

  new aws.iam.RolePolicyAttachment("dynamodb-execute-policy-attachment", {
    role: role.name,
    policyArn: lambdaDynamoDbPolicy.arn,
  });

  new aws.iam.RolePolicyAttachment("sqs-execute-policy-attachment", {
    role: role.name,
    policyArn: lambdaSQSPolicy.arn,
  });

  return role;
}
