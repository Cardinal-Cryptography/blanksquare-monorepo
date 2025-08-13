terraform {
  source = "git@github.com:Cardinal-Cryptography/tf-modules-prover-server.git?ref=v1.0.2"

  extra_arguments "conditional_vars" {
    commands = [
      "apply",
      "plan",
      "import",
      "push",
      "refresh"
    ]
    optional_var_files = [
      "terraform.tfvars"
    ]
  }
}

locals {
  project_name = "prover-server"
  environment = "ci"
  aws_region = "eu-west-1"
}

inputs = {
    aws_region = local.aws_region
    environment = local.environment
    project_name = local.project_name
}

generate "provider" {
  path      = "provider.tf"
  if_exists = "overwrite_terragrunt"
  contents  = <<EOF
provider "aws" {
  region = "${local.aws_region}"
}
EOF
}

remote_state {
  backend = "s3"
  config = {
    bucket         = "${local.project_name}-terraformstate-demo"
    key            = "${local.environment}/terraform.tfstate"
    region         = "eu-west-1"
    dynamodb_table = "${local.project_name}-tfstate"
    encrypt        = true
  }
  generate = {
    path      = "backend.tf"
    if_exists = "overwrite_terragrunt"
  }
}
