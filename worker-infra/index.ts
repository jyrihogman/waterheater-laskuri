import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import { provisionMonthlyPricingWorker } from "./monthly-pricing-worker";
import { createAlarmSubscription } from "./alarms";
import { provisionQueues } from "./queues";
import { provisionWorker } from "./worker";
import { provisionMessageHandler } from "./message-handler";

const config = new pulumi.Config();
const alarmEmail = config.require("alarmEmail");

const commonTags = {
  Service: "waterheater-calc-worker",
};

export type Tags = typeof commonTags;

const { queue, proxyDLQ, dlq } = provisionQueues(commonTags);
const alarmTopic = createAlarmSubscription(alarmEmail);

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

provisionWorker(queue, dlq, eventRole, dynamoTable, alarmTopic, commonTags);
provisionMessageHandler(queue, proxyDLQ, eventRole, alarmTopic, commonTags);
provisionMonthlyPricingWorker(eventRole, alarmTopic, config, commonTags);
