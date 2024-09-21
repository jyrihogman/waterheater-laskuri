import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const commonTags = {
  Service: "waterheater-calc-service",
};

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

new aws.iam.RolePolicyAttachment("dynamodb-readonly-policy-attachment", {
  role: lambdaRole.name,
  policyArn: aws.iam.ManagedPolicy.AmazonDynamoDBReadOnlyAccess,
});

const image = aws.ecr.getImageOutput({
  repositoryName: "waterheater-calc",
  imageTag: "latest",
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
