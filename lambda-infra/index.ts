import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const config = new pulumi.Config();
const alarmEmail = config.require("alarmEmail");

const commonTags = {
  Service: "waterheater-calc-worker",
};

const queue = new aws.sqs.Queue("getPricingEventQueue", {
  tags: commonTags,
  name: "GetPricingEventQueue",
  visibilityTimeoutSeconds: 120,
});

const dlq = new aws.sqs.Queue("getPricingEventDLQ", {
  name: "GetPricingEventDLQ",
  messageRetentionSeconds: 1209600, // 14 days
  visibilityTimeoutSeconds: 120,
});

const proxyDLQ = new aws.sqs.Queue("getPricingEventProxyDLQ", {
  tags: commonTags,
  name: "GetPricingEventProxyDLQ",
  visibilityTimeoutSeconds: 120,
  redriveAllowPolicy: pulumi.jsonStringify({
    redrivePermission: "byQueue",
    sourceQueueArns: [queue.arn],
  }),
});

new aws.sqs.RedrivePolicy("queueRedrivePolicy", {
  queueUrl: queue.id,
  redrivePolicy: pulumi.jsonStringify({
    deadLetterTargetArn: proxyDLQ.arn,
    maxReceiveCount: 1,
  }),
});

const eventRole = new aws.iam.Role("eventRole", {
  tags: commonTags,
  assumeRolePolicy: JSON.stringify({
    Version: "2012-10-17",
    Statement: [
      {
        Action: "sts:AssumeRole",
        Effect: "Allow",
        Principal: {
          Service: "events.amazonaws.com",
        },
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

new aws.iam.RolePolicy("eventRolePolicy", {
  role: eventRole.id,
  policy: queue.arn.apply((arn) =>
    JSON.stringify({
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Action: "sqs:SendMessage",
          Resource: arn,
        },
      ],
    }),
  ),
});

new aws.scheduler.Schedule("dailyMessageSchedule", {
  name: "GetElectricityPricingSchedule",
  groupName: "default",
  flexibleTimeWindow: {
    mode: "OFF",
  },
  scheduleExpression: "cron(0 * * * ? *)",
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

const dynamoTable = new aws.dynamodb.Table("electricityPricingData", {
  name: "electricity_pricing",
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

const alarmTopic = new aws.sns.Topic("alarmTopic", {
  displayName: "Worker Lambda Alarm Topic",
});

new aws.sns.TopicSubscription("emailSubscription", {
  topic: alarmTopic.arn,
  protocol: "email",
  endpoint: alarmEmail,
});

const worker = new aws.lambda.Function("waterheater-calc-pricing-worker", {
  tags: commonTags,
  code: new pulumi.asset.AssetArchive({
    bootstrap: new pulumi.asset.FileAsset("../target/lambda/worker/bootstrap"),
  }),
  handler: "bootstrap",
  runtime: aws.lambda.Runtime.CustomAL2023,
  role: createWorkerRole(dynamoTable, queue).arn,
  timeout: 60,
});

const messageHandler = new aws.lambda.Function("message-retry-handler", {
  tags: commonTags,
  code: new pulumi.asset.AssetArchive({
    bootstrap: new pulumi.asset.FileAsset(
      "../target/lambda/message-handler/bootstrap",
    ),
  }),
  handler: "bootstrap",
  environment: {
    variables: {
      queueArn: queue.arn,
      roleArn: eventRole.arn,
    },
  },
  runtime: aws.lambda.Runtime.CustomAL2023,
  role: createMessageHandlerRole(queue, proxyDLQ).arn,
  timeout: 60,
});

createLambdaAlarms(worker, "Worker", alarmTopic);
createLambdaAlarms(messageHandler, "MessageHandler", alarmTopic);

new aws.lambda.Permission("sqsInvokeWorker", {
  action: "lambda:InvokeFunction",
  function: worker.name,
  principal: "sqs.amazonaws.com",
  sourceArn: queue.arn,
});

new aws.lambda.Permission("sqsInvokeMessageHandler", {
  action: "lambda:InvokeFunction",
  function: messageHandler.name,
  principal: "sqs.amazonaws.com",
  sourceArn: proxyDLQ.arn,
});

// Create an event source mapping to trigger the Lambda from the SQS queue
new aws.lambda.EventSourceMapping("sqsWorkerLambdaTrigger", {
  eventSourceArn: queue.arn,
  functionName: worker.arn,
  batchSize: 1, // Process one message at a time
});

new aws.lambda.EventSourceMapping("sqsMessageHandlerLambdaTrigger", {
  eventSourceArn: proxyDLQ.arn,
  functionName: messageHandler.arn,
  batchSize: 1, // Process one message at a time
});

function createWorkerRole(
  dynamoTable: aws.dynamodb.Table,
  queue: aws.sqs.Queue,
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

function createMessageHandlerRole(
  queue: aws.sqs.Queue,
  proxyDLQ: aws.sqs.Queue,
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

function createLambdaAlarms(
  lambda: aws.lambda.Function,
  resourceName: string,
  alarmTopic: aws.sns.Topic,
) {
  new aws.cloudwatch.MetricAlarm(`${resourceName}InvocationsAlarm`, {
    name: `${resourceName}-invocation-alarm`,
    alarmDescription:
      "Alarm when worker Lambda function invocations exceed the threshold",
    comparisonOperator: "GreaterThanThreshold",
    evaluationPeriods: 1,
    threshold: 10,
    metricName: "Invocations",
    namespace: "AWS/Lambda",
    statistic: "Sum",
    period: 3600, // 1 hour
    dimensions: {
      FunctionName: lambda.name,
    },
    alarmActions: [alarmTopic.arn],
  });

  new aws.cloudwatch.MetricAlarm(`${resourceName}DurationAlarm`, {
    name: `${resourceName}-duration-alarm`,
    alarmDescription: "Alarm when 99th percentile duration exceeds 5 seconds",
    comparisonOperator: "GreaterThanThreshold",
    evaluationPeriods: 1,
    threshold: 5000,
    metricName: "Duration",
    namespace: "AWS/Lambda",
    extendedStatistic: "p99",
    period: 300, // 5 minutes
    dimensions: {
      FunctionName: lambda.name,
    },
    alarmActions: [alarmTopic.arn],
  });
}

export const workerName = worker.name;
export const workerArn = worker.arn;
export const queueUrl = queue.url;
export const finishedTime = new Date().toISOString();
