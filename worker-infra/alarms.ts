import * as aws from "@pulumi/aws";

export function createAlarmSubscription(alarmEmail: string) {
  const alarmTopic = new aws.sns.Topic("alarmTopic", {
    displayName: "Worker Lambda Alarm Topic",
  });

  new aws.sns.TopicSubscription("emailSubscription", {
    topic: alarmTopic.arn,
    protocol: "email",
    endpoint: alarmEmail,
  });

  return alarmTopic;
}

export function createLambdaAlarms(
  lambda: aws.lambda.Function,
  resourceName: string,
  alarmTopic: aws.sns.Topic,
) {
  new aws.cloudwatch.MetricAlarm(`${resourceName}InvocationsAlarm`, {
    name: `${resourceName}-invocation-alarm`,
    alarmDescription:
      "Alarm when worker Lambda function invocations exceed the threshold",
    comparisonOperator: "GreaterThanThreshold",
    evaluationPeriods: 1,
    threshold: 10,
    metricName: "Invocations",
    namespace: "AWS/Lambda",
    statistic: "Sum",
    period: 3600, // 1 hour
    dimensions: {
      FunctionName: lambda.name,
    },
    alarmActions: [alarmTopic.arn],
  });

  new aws.cloudwatch.MetricAlarm(`${resourceName}DurationAlarm`, {
    name: `${resourceName}-duration-alarm`,
    alarmDescription: "Alarm when 99th percentile duration exceeds 5 seconds",
    comparisonOperator: "GreaterThanThreshold",
    evaluationPeriods: 1,
    threshold: 5000,
    metricName: "Duration",
    namespace: "AWS/Lambda",
    extendedStatistic: "p99",
    period: 300, // 5 minutes
    dimensions: {
      FunctionName: lambda.name,
    },
    alarmActions: [alarmTopic.arn],
  });
}
