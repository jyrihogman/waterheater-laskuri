import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

import { Runtime } from "@pulumi/aws/lambda";

const worker = new aws.lambda.Function("waterheater-calc-pricing-worker", {
  code: new pulumi.asset.AssetArchive({
    bootstrap: new pulumi.asset.FileAsset(
      "../target/lambda/waterheater-calc-worker/bootstrap",
    ),
  }),
  handler: "bootstrap",
  runtime: Runtime.CustomAL2023,
  // packageType: "Image",
  // imageUri: getImageFromECR().imageUri,
  role: createIAMRole().arn,
  timeout: 60,
});

const cronRule = new aws.cloudwatch.EventRule("waterheater-calc-cron-rule", {
  scheduleExpression: "cron(0 18 * * ? *)", // Run at 18:00 UTC each day
  description: "Triggers waterheater-calc worker at 18:00 UTC every day.",
});

new aws.cloudwatch.EventTarget("waterheater-calc-cron-target", {
  rule: cronRule.name,
  arn: worker.arn,
});

new aws.lambda.Permission("waterheater-calc-cloudwatch-permission", {
  action: "lambda:InvokeFunction",
  principal: "events.amazonaws.com",
  function: worker.name,
  sourceArn: cronRule.arn,
});

function getImageFromECR() {
  return aws.ecr.getImageOutput({
    repositoryName: "waterheater-calc",
    imageTag: "lambda:latest",
  });
}

function createIAMRole() {
  const role = new aws.iam.Role("waterheater-calc-worker-role", {
    assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
      Service: "lambda.amazonaws.com",
    }),
  });

  new aws.iam.RolePolicyAttachment("lambda-execute-policy-attachment", {
    role: role.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaExecute,
  });

  new aws.iam.RolePolicyAttachment("lambda-dynamodb-policy-attachment", {
    role: role.name,
    policyArn: aws.iam.ManagedPolicy.AWSLambdaDynamoDBExecutionRole,
  });

  return role;
}

export const workerName = worker.name;
export const workerArn = worker.arn;
