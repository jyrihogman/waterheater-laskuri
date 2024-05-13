import * as aws from "@pulumi/aws";

const role = new aws.iam.Role("waterheater-calc-apprunner-role", {
  name: "waterheater-calc-apprunner-role",
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: "tasks.apprunner.amazonaws.com",
  }),
});

new aws.iam.RolePolicyAttachment("ecs-lb-policy-attachment", {
  role,
  policyArn: aws.iam.ManagedPolicy.AmazonDynamoDBReadOnlyAccess,
});

const waterHeaterCalcAsc = new aws.apprunner.AutoScalingConfigurationVersion(
  "waterheater-calc-asc",
  {
    autoScalingConfigurationName: "waterheater-calc-asc",
    maxConcurrency: 200,
    maxSize: 2,
    minSize: 1,
    tags: {
      Name: "waterheater-calc",
    },
  },
);

new aws.apprunner.Service("serviceResource", {
  serviceName: "waterheater-calc-apprunner",
  sourceConfiguration: {
    autoDeploymentsEnabled: false,
    imageRepository: {
      imageConfiguration: {
        port: "8001",
      },
      imageIdentifier: "public.ecr.aws/s1w6z3w3/waterheater-calc:latest",
      imageRepositoryType: "ECR_PUBLIC",
    },
  },
  autoScalingConfigurationArn: waterHeaterCalcAsc.arn,
  healthCheckConfiguration: {
    healthyThreshold: 1,
    interval: 20,
    protocol: "TCP",
    timeout: 5,
    unhealthyThreshold: 5,
  },
  instanceConfiguration: {
    cpu: "256",
    instanceRoleArn: role.arn,
    memory: "512",
  },
  networkConfiguration: {
    ingressConfiguration: {
      isPubliclyAccessible: true,
    },
  },
  observabilityConfiguration: {
    observabilityEnabled: false,
  },
  tags: {
    string: "waterheater-calc",
  },
});
