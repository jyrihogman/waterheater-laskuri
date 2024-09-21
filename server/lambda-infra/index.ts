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
  packageType: "Image",
  imageUri: image.imageUri,
  role: lambdaRole.arn,
  timeout: 30,
  tags: commonTags,
});

const apigw = new aws.apigateway.RestApi("waterheater-calc-apigw", {
  name: "waterheater-calc-apigw",
});

new aws.lambda.Permission("invokePermission", {
  action: "lambda:InvokeFunction",
  function: lambdaFunction.arn,
  principal: "apigateway.amazonaws.com",
  sourceArn: pulumi.interpolate`${apigw.executionArn}/*/*/*`,
});

const rootResource = new aws.apigateway.Resource("status", {
  restApi: apigw.id,
  parentId: apigw.rootResourceId,
  pathPart: "",
});

const rootMethod = new aws.apigateway.Method("rootMethod", {
  restApi: apigw.id,
  resourceId: rootResource.id,
  httpMethod: "ANY",
  authorization: "NONE",
});

const lambdaIntegration = new aws.apigateway.Integration("lambdaIntegration", {
  restApi: apigw.id,
  resourceId: rootResource.id,
  httpMethod: "ANY",
  integrationHttpMethod: "POST",
  type: "AWS_PROXY",
  uri: lambdaFunction.invokeArn,
});

const deployment = new aws.apigateway.Deployment("apiDeployment", {
  restApi: apigw.id,
  stageName: "prod",
  description: "Production deployment",
  triggers: {
    redeployment: pulumi
      .all([rootMethod.id, lambdaIntegration.id])
      .apply(() => Date.now().toString()),
  },
});

export const apiEndpoint = pulumi.interpolate`${deployment.invokeUrl}`;
