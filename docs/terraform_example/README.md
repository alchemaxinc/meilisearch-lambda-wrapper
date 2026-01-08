# Terraform Infrastructure as Code

This is an example Terraform project that sets up AWS infrastructure for running Meilisearch on Lambda with EFS storage.
It includes som additional logic such as an SNS topic with metrics and alerting.

It assumes S3 for the state backend and has a `.tfvars` file attached for example configuration.

## Pre-requisites

In order to ensure shared Terraform state, run these manual commands in all environments needed first:

```bash
export AWS_REGION=<your-region>
export AWS_PROFILE=<your-profile>
```

```bash
aws s3api create-bucket \
  --bucket <some-unique-bucket-name> \
  --region <your-region> \
  --create-bucket-configuration LocationConstraint=<your-region>
```

... and maybe a `-production-` version for production. Consider adding a random postfix to ensure uniqueness.

If you want, though optional:

```bash
aws s3api put-bucket-versioning \
  --bucket <some-unique-bucket-name> \
  --versioning-configuration \
  Status=Enabled
```

## If you suddenly get Terraform State Lock Errors

Something messed up, you probably ran multiple CCI jobs with SSH on, or something. Force-unlock it by running:

```bash
terraform force-unlock -force << Lock Info ID >>
```

The Lock Info ID is the one listed in the message you get, such as

```bash
│ Error message: operation error S3: PutObject, https response error StatusCode: 412, RequestID: Y97P1RR26W3H41JE, HostID: PK4+i4Jabearjo1eW5KfooinZpMudh3qLJOcF1fl0o40Od2f/sZW2DnV4KlFqmDKt1B0c5C+eV6rf0KYQPqnPJ54n8QIerFa, api error PreconditionFailed: At least one of the pre-conditions you specified did not hold
│ Lock Info:
│   ID:        4140490f-df2f-2b5e-1f7b-8bf3fe42f398
│   Path:      <some-unique-bucket-name>/envs/terraform.tfstate
│   Operation: OperationTypePlan
│   Who:       circleci@959fcba5e7a7
│   Version:   1.12.2
│   Created:   2025-07-15 20:47:45.613267109 +0000 UTC
│   Info:      ...... etc
```
