import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
import type { Tags } from "./index";

export function provisionQueues(commonTags: Tags) {
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

  new aws.sqs.RedrivePolicy("dlqRedrivePolicy", {
    queueUrl: proxyDLQ.id,
    redrivePolicy: pulumi.jsonStringify({
      deadLetterTargetArn: dlq.arn,
      maxReceiveCount: 1,
    }),
  });

  return {
    queue,
    dlq,
    proxyDLQ,
  };
}
