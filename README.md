Http server calculating the cheapest x hour period for electricity.

Application is deservedly deployed as a container in an `ECS` cluster.
Infra is developed with `Pulumi` and resides in `pulumi-infra` directory.

Plan is to create add a lambda which will be triggered by a cron job to fetch
pricing data from an API and insert it into a database accessible by the `EC2` instances in the cluster.
