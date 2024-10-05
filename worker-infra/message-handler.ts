import * as aws from "@pulumi/aws";
import type { Tags } from "./index";
import { createLambdaAlarms } from "./alarms";

export function provisionMessageHandler(
  queue: aws.sqs.Queue,
  proxyDLQ: aws.sqs.Queue,
  eventRole: aws.iam.Role,
  alarmTopic: aws.sns.Topic,
  commonTags: Tags,
) {
  const messageHandlerOutput = aws.ecr.getImageOutput({
    repositoryName: "waterheater-calc-msg-handler",
    imageTag: "latest",
  });

  const messageHandler = new aws.lambda.Function("message-retry-handler", {
    name: "wh-message-retry-handler",
    tags: commonTags,
    imageUri: messageHandlerOutput.imageUri,
    packageType: "Image",
    environment: {
      variables: {
        queueArn: queue.arn,
        roleArn: eventRole.arn,
      },
    },
    role: createMessageHandlerRole(queue, proxyDLQ, commonTags).arn,
    timeout: 60,
  });

  createLambdaAlarms(messageHandler, "MessageHandler", alarmTopic);

  new aws.lambda.Permission("sqsInvokeMessageHandler", {
    action: "lambda:InvokeFunction",
    function: messageHandler.name,
    principal: "sqs.amazonaws.com",
    sourceArn: proxyDLQ.arn,
  });

  new aws.lambda.EventSourceMapping("sqsMessageHandlerLambdaTrigger", {
    eventSourceArn: proxyDLQ.arn,
    functionName: messageHandler.arn,
    batchSize: 1, // Process one message at a time
  });

  return messageHandler;
}

function createMessageHandlerRole(
  queue: aws.sqs.Queue,
  proxyDLQ: aws.sqs.Queue,
  commonTags: Tags,
) {
  const eventBridgePolicy = new aws.iam.Policy("eventBridgePolicy", {
    description: "Policy to allow Lambda to interact with EventBridge",
    policy: JSON.stringify({
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Action: [
            "events:PutTargets",
            "events:PutRule",
            "events:DescribeRule",
            "scheduler:CreateSchedule",
            "iam:PassRole",
          ],
          Resource: "*", // Scope down as needed to target specific resources
        },
      ],
    }),
  });

  const lambdaSQSPolicy = new aws.iam.Policy("messageHandlerSQSPolicy", {
    tags: commonTags,
    description: "IAM policy for Lambda to Send SQS Messages",
    policy: queue.arn.apply((arn) =>
      JSON.stringify({
        Version: "2012-10-17",
        Statement: [
          {
            Effect: "Allow",
            Action: ["sqs:SendMessage", "sqs:GetQueueAttributes"],
            Resource: arn,
          },
        ],
      }),
    ),
  });

  const lambdaDLQPolicy = new aws.iam.Policy("messageHandlerDLQPolicy", {
    tags: commonTags,
    description: "IAM policy for Lambda to Receive and Delete SQS Messages",
    policy: proxyDLQ.arn.apply((arn) =>
      JSON.stringify({
        Version: "2012-10-17",
        Statement: [
          {
            Effect: "Allow",
            Action: [
              "sqs:ReceiveMessage",
              "sqs:DeleteMessage",
              "sqs:GetQueueAttributes",
            ],
            Resource: arn,
          },
        ],
      }),
    ),
  });

  const role = new aws.iam.Role("messageHandlerWorkerRole", {
    tags: commonTags,
    assumeRolePolicy: JSON.stringify({
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Principal: {
            Service: "lambda.amazonaws.com",
          },
          Action: "sts:AssumeRole",
        },
        {
          Effect: "Allow",
          Principal: {
            Service: "scheduler.amazonaws.com",
          },
          Action: "sts:AssumeRole",
        },
      ],
    }),
  });

  new aws.iam.RolePolicyAttachment("messageHandlerLambdaPolicyAttachment", {
    role: role.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaExecute,
  });

  new aws.iam.RolePolicyAttachment("messageHandlerSQSExecutePolicyAttachment", {
    role: role.name,
    policyArn: lambdaSQSPolicy.arn,
  });

  new aws.iam.RolePolicyAttachment("messageHandlerDLQExecutePolicyAttachment", {
    role: role.name,
    policyArn: lambdaDLQPolicy.arn,
  });

  new aws.iam.RolePolicyAttachment("messageHandlerEBPolicyAttachment", {
    role: role.name,
    policyArn: eventBridgePolicy.arn,
  });

  return role;
}
