import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as pulumi from "@pulumi/pulumi";

const logGroup = new aws.cloudwatch.LogGroup("waterheater-calc-logs", {
  retentionInDays: 14,
  skipDestroy: false,
  namePrefix: "waterheater-calc-logs",
});

const vpc = new awsx.ec2.DefaultVpc("defaultVpc");
const clusterName = "waterheater-calc-cluster2";
const cluster = new aws.ecs.Cluster("waterheater-calc-cluster2", {
  name: clusterName,
});

const sg = new aws.ec2.SecurityGroup("waterheater-calc-sg", {
  vpcId: vpc.vpcId,
  description:
    "The Security group for all instances that only allow ingress of the Load Balancer.",
  ingress: [
    {
      fromPort: 80,
      toPort: 9000,
      protocol: "tcp",
      cidrBlocks: ["0.0.0.0/0"],
    },
  ],
  egress: [
    { fromPort: 1, toPort: 65535, protocol: "tcp", cidrBlocks: ["0.0.0.0/0"] },
  ],
});

const lb = new awsx.lb.ApplicationLoadBalancer("waterheater-calc-alb", {
  securityGroups: [sg.id],
  subnetIds: [vpc.publicSubnetIds[0], vpc.publicSubnetIds[1]],
  defaultTargetGroup: {
    port: 80,
    protocol: "HTTP",
    targetType: "instance",
    vpcId: vpc.vpcId,
    // healthCheck: {
    //   healthyThreshold: 2,
    //   unhealthyThreshold: 2,
    //   timeout: 5,
    //   interval: 50,
    //   path: "/",
    // },
  },
});

const ecsInstanceRole = new aws.iam.Role("waterheater-calc-ecs-instance-role", {
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: "ec2.amazonaws.com",
  }),
});
new aws.iam.RolePolicyAttachment("ecs-ec2-policy-attachment", {
  role: ecsInstanceRole,
  policyArn: aws.iam.ManagedPolicy.AmazonEC2ContainerServiceforEC2Role,
});
new aws.iam.RolePolicyAttachment("ecs-lb-policy-attachment", {
  role: ecsInstanceRole,
  policyArn: aws.iam.ManagedPolicy.ElasticLoadBalancingFullAccess,
});
new aws.iam.RolePolicyAttachment("ecs-ecr-policy-attachment", {
  role: ecsInstanceRole,
  policyArn:
    aws.iam.ManagedPolicy.AmazonElasticContainerRegistryPublicFullAccess,
});
const ecsInstanceProfile = new aws.iam.InstanceProfile(
  "waterheater-calc-instance-profile",
  {
    role: ecsInstanceRole.name,
  },
);

const taskDefinition = new awsx.ecs.EC2TaskDefinition(
  "waterheater-calc-app-taskdef",
  {
    memory: "256",
    family: "waterheater-calc-app-tasdef",
    networkMode: "bridge",
    taskRole: {
      roleArn: new aws.iam.Role("waterheater-calc-ecs-task-role", {
        assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
          Service: "ecs-tasks.amazonaws.com",
        }),
      }).arn,
    },
    container: {
      name: "waterheater-calc-ecs-container",
      memory: 256,
      essential: true,
      logConfiguration: {
        logDriver: "awslogs",
        options: {
          "awslogs-region": "us-east-1",
          "awslogs-group": logGroup.name,
          "awslogs-stream-prefix": "ecs-container",
        },
      },
      image: "public.ecr.aws/s1w6z3w3/waterheater-calc:latest",
      portMappings: [
        {
          containerPort: 8001,
          hostPort: 8001,
          targetGroup: lb.defaultTargetGroup,
        },
      ],
    },
  },
);

const ami = pulumi.output(
  aws.ec2.getAmi({
    owners: ["amazon"],
    mostRecent: true,
    filters: [
      {
        name: "name",
        // ECS-Optimized AMI
        values: ["amzn2-ami-ecs-hvm-*-x86_64-ebs"],
      },
    ],
  }),
);

const ec2Instance = new aws.ec2.Instance("waterheater-calc-ec2-instance", {
  instanceType: "t2.micro",
  ami: ami.id,
  vpcSecurityGroupIds: [sg.id],
  iamInstanceProfile: ecsInstanceProfile.name,
  userData: pulumi.interpolate`#!/bin/bash
    echo ECS_CLUSTER=${clusterName} >> /etc/ecs/ecs.config`,
  tags: {
    name: "waterheater-calc-ec2-instance",
  },
});

new awsx.ecs.EC2Service("waterheater-calc-ecs-ec2-service", {
  cluster: cluster.id,
  desiredCount: 1,
  deploymentMinimumHealthyPercent: 0,
  deploymentMaximumPercent: 100,
  taskDefinition: taskDefinition.taskDefinition.arn,
});

new aws.lb.TargetGroupAttachment("test", {
  targetGroupArn: lb.defaultTargetGroup.arn,
  targetId: ec2Instance.id,
  port: 8001,
});

export const url = pulumi.interpolate`http://${lb.loadBalancer.dnsName}`;
