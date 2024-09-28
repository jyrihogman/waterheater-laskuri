import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const config = new pulumi.Config();
const redisUrl = config.require("redis_url");

const commonTags = {
  Service: "waterheater-calc-service",
};

const vpc = new aws.ec2.Vpc("waterheater-calc-vpc", {
  // VPC with 255 IP addresses
  cidrBlock: "10.0.0.0/24",
  enableDnsSupport: true,
  enableDnsHostnames: true,
  tags: { Name: "customVpc" },
});

const privateSubnet = new aws.ec2.Subnet("publicSubnet-1", {
  // Subnet with 255 IP addresses
  cidrBlock: "10.0.1.0/24",
  vpcId: vpc.id,
  availabilityZone: "eu-north-1a",
  tags: { Name: "privateSubnet" },
});

const privateSubnet2 = new aws.ec2.Subnet("publicSubnet-2", {
  // Subnet with 255 IP addresses
  cidrBlock: "10.0.2.0/24",
  vpcId: vpc.id,
  availabilityZone: "eu-north-1b",
  tags: { Name: "privateSubnet" },
});

// VPC Gateway Endpoint for lambdas to be able to connect to DynamoDB
new aws.ec2.VpcEndpoint("vpcEndpoint", {
  vpcId: vpc.id,
  serviceName: "com.amazonaws.eu-north-1.dynamodb",
  routeTableIds: [vpc.mainRouteTableId],
  vpcEndpointType: "Gateway",
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

const lambdaRole = new aws.iam.Role("waterheater-calc-lambda-role", {
  name: "waterheater-calc-lambda-role",
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: "lambda.amazonaws.com",
  }),
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
      "../target/lambda/waterheater-calc/bootstrap",
    ),
  }),
  handler: "bootstrap",
  runtime: aws.lambda.Runtime.CustomAL2023,
  environment: {
    variables: {
      REDIS_ENDPOINT: redisUrl,
      DEPLOY_ENV: "production",
    },
  },
  vpcConfig: {
    securityGroupIds: [lambdaSecurityGroup.id],
    subnetIds: [privateSubnet.id, privateSubnet2.id],
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
