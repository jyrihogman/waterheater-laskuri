import * as awsx from "@pulumi/awsx";
import * as eks from "@pulumi/eks";
import * as k8s from "@pulumi/kubernetes";

const eksVpc = new awsx.ec2.Vpc("wh-calc-vpc", {
  enableDnsHostnames: true,
  cidrBlock: "10.0.0.0/16",
});

const eksCluster = new eks.Cluster("wh-calc-eks-cluster", {
  vpcId: eksVpc.vpcId,
  publicSubnetIds: eksVpc.publicSubnetIds,
  privateSubnetIds: eksVpc.privateSubnetIds,
  instanceType: "t3.micro",
  desiredCapacity: 1,
  minSize: 1,
  maxSize: 1,
  nodeAssociatePublicIpAddress: false,
  endpointPrivateAccess: false,
  endpointPublicAccess: true,
});

new eks.NodeGroup(
  "wh-calc-node-group",
  {
    cluster: eksCluster,
    instanceType: "t3.micro",
  },
  {
    providers: { kubernetes: eksCluster.provider },
  },
);

const k8sProvider = new k8s.Provider("wh-calc-k8s-provider", {
  kubeconfig: eksCluster.kubeconfig,
});

const appLabels = { app: "waterheater-calc-applabel" };

const deployment = new k8s.apps.v1.Deployment(
  "waterheater-calc-deployment",
  {
    spec: {
      selector: { matchLabels: appLabels },
      replicas: 1,
      template: {
        metadata: { labels: appLabels },
        spec: {
          containers: [
            {
              name: "waterheater-calc",
              image: "public.ecr.aws/s1w6z3w3/waterheater-calc:latest",
              ports: [{ containerPort: 8001 }],
            },
          ],
        },
      },
    },
  },
  { provider: k8sProvider },
);

const service = new k8s.core.v1.Service(
  "waterheater-calc-service",
  {
    metadata: {
      labels: appLabels,
    },
    spec: {
      type: "LoadBalancer",
      selector: appLabels,
      ports: [{ port: 80, targetPort: 8001 }],
    },
  },
  { provider: k8sProvider },
);

export const kubeconfig = eksCluster.kubeconfig;
export const deploymentName = deployment.metadata.name;
export const vpcId = eksVpc.vpcId;
export const serviceName = service.metadata.name;
export const serviceIp = service.status.loadBalancer.ingress[0].ip;
