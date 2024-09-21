import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const commonTags = {
  Service: "wateheater-calc-service",
};

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
    tags: commonTags,
    autoScalingConfigurationName: "waterheater-calc-asc",
    maxConcurrency: 200,
    maxSize: 2,
    minSize: 1,
  },
);

const image = pulumi.output(
  aws.ecr.getImage({
    repositoryName: "waterheater-calc",
    imageTag: "latest",
  }),
);

const imageIdentifier = image.apply(
  (img) =>
    pulumi.interpolate`${img.registryId}.dkr.ecr.us-east-1.amazonaws.com/${img.repositoryName}:latest`,
);

new aws.apprunner.Service("serviceResource", {
  serviceName: "waterheater-calc-apprunner",
  sourceConfiguration: {
    autoDeploymentsEnabled: false,
    imageRepository: {
      imageConfiguration: {
        port: "8001",
      },
      imageIdentifier,
      imageRepositoryType: "ECR",
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
  tags: commonTags,
});
