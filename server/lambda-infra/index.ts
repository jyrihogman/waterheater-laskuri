import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const commonTags = {
  Service: "waterheater-calc-service",
};

const vpc = new aws.ec2.Vpc("waterheater-calc-vpc", {
  // VPC with 65,536 IP addresses
  cidrBlock: "10.0.0.0/16",
  enableDnsSupport: true,
  enableDnsHostnames: true,
  tags: { Name: "customVpc" },
});

const publicSubnet = new aws.ec2.Subnet("publicSubnet-1", {
  // Subnet with 256 IP addresses
  cidrBlock: "10.0.1.0/24",
  vpcId: vpc.id,
  availabilityZone: "eu-north-1a",
  mapPublicIpOnLaunch: true,
  tags: { Name: "publicSubnet" },
});

const publicSubnet2 = new aws.ec2.Subnet("publicSubnet-2", {
  // Subnet with 256 IP addresses
  cidrBlock: "10.0.2.0/24",
  vpcId: vpc.id,
  availabilityZone: "eu-north-1b",
  mapPublicIpOnLaunch: true,
  tags: { Name: "publicSubnet" },
});

const lambdaSecurityGroup = new aws.ec2.SecurityGroup("lambda-sg", {
  vpcId: vpc.id,
  ingress: [
    // Allow Redis traffic
    {
      protocol: "tcp",
      fromPort: 6379,
      toPort: 6379,
      cidrBlocks: [vpc.cidrBlock],
    },
  ],
  egress: [
    // Allow everything outbound
    { protocol: "-1", fromPort: 0, toPort: 0, cidrBlocks: ["0.0.0.0/0"] },
  ],
  tags: { Name: "lambda-sg" },
});

const redis = new aws.elasticache.ServerlessCache("waterheater-calc-redis", {
  engine: "redis",
  name: "waterheater-calc-redis",
  securityGroupIds: [lambdaSecurityGroup.id],
  subnetIds: [publicSubnet.id, publicSubnet2.id],
  cacheUsageLimits: {
    dataStorage: {
      maximum: 2,
      unit: "GB",
    },
    ecpuPerSeconds: [
      {
        maximum: 1000,
      },
    ],
  },
  dailySnapshotTime: "09:00",
  description: "Server Elasticache",
  majorEngineVersion: "7",
  snapshotRetentionLimit: 1,
});

const lambdaRole = new aws.iam.Role("waterheater-calc-lambda-role", {
  name: "waterheater-calc-lambda-role",
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: "lambda.amazonaws.com",
  }),
  inlinePolicies: [
    {
      name: "elasticache-access",
      policy: pulumi.jsonStringify({
        Version: "2012-10-17",
        Statement: [
          {
            Action: ["elasticache:Connect"],
            Effect: "Allow",
            Resource: [redis.arn],
          },
        ],
      }),
    },
  ],
});

new aws.iam.RolePolicyAttachment("lambda-basic-execution-role", {
  role: lambdaRole.name,
  policyArn: aws.iam.ManagedPolicy.AWSLambdaBasicExecutionRole,
});

new aws.iam.RolePolicyAttachment("lambda-vpc-execution-role", {
  role: lambdaRole.name,
  policyArn: aws.iam.ManagedPolicy.AWSLambdaVPCAccessExecutionRole,
});

new aws.iam.RolePolicyAttachment("dynamodb-readonly-policy-attachment", {
  role: lambdaRole.name,
  policyArn: aws.iam.ManagedPolicy.AmazonDynamoDBReadOnlyAccess,
});

const lambdaFunction = new aws.lambda.Function("waterheater-calc-lambda", {
  name: "waterheater-calc-lambda",
  code: new pulumi.asset.AssetArchive({
    bootstrap: new pulumi.asset.FileAsset(
      "../../target/lambda/waterheater-calc/bootstrap",
    ),
  }),
  handler: "bootstrap",
  runtime: aws.lambda.Runtime.CustomAL2023,
  environment: {
    variables: {
      REDIS_ENDPOINTS: redis.endpoints.apply(
        (a) => `${a[0].address}:${a[0].port}`,
      ),
      DEPLOY_ENV: "production",
    },
  },
  vpcConfig: {
    securityGroupIds: [lambdaSecurityGroup.id],
    subnetIds: [publicSubnet.id, publicSubnet2.id],
  },
  role: lambdaRole.arn,
  timeout: 30,
  tags: commonTags,
});

const httpApi = new aws.apigatewayv2.Api("waterheater-calczz", {
  name: "waterheater-calc-http-api",
  protocolType: "HTTP",
  tags: commonTags,
});

const lambdaIntegration = new aws.apigatewayv2.Integration(
  "lambdaIntegration",
  {
    apiId: httpApi.id,
    integrationType: "AWS_PROXY",
    integrationUri: lambdaFunction.invokeArn,
    payloadFormatVersion: "2.0",
  },
);

const anyProxyRoute = new aws.apigatewayv2.Route("anyProxyRoute", {
  apiId: httpApi.id,
  routeKey: "ANY /{proxy+}",
  target: pulumi.interpolate`integrations/${lambdaIntegration.id}`,
});

const rootRoute = new aws.apigatewayv2.Route("rootRoute", {
  apiId: httpApi.id,
  routeKey: "ANY /",
  target: pulumi.interpolate`integrations/${lambdaIntegration.id}`,
});

new aws.apigatewayv2.Deployment("apiDeployment", {
  apiId: httpApi.id,
  description: "Deployment for all routes",
  triggers: {
    redeployment: pulumi
      .all([anyProxyRoute.id, rootRoute.id, lambdaIntegration.id])
      .apply(() => Date.now().toString()),
  },
});

new aws.apigatewayv2.Stage("apiStage", {
  apiId: httpApi.id,
  name: "$default",
  autoDeploy: true,
  tags: commonTags,
});

new aws.lambda.Permission("apiGatewayLambdaPermission", {
  action: "lambda:InvokeFunction",
  function: lambdaFunction.name,
  principal: "apigateway.amazonaws.com",
  sourceArn: pulumi.interpolate`${httpApi.executionArn}/*/*`,
});

export const apiEndpoint = pulumi.interpolate`${httpApi.apiEndpoint}`;
