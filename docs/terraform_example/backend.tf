# Read `README.md` for pre-requisites and instructions before this backend configuration will work.
terraform {
  backend "s3" {
    # Bucket name is injected at init-time since it cannot be the same for dev and prod, when
    # these are in different AWS accounts.
    key          = "envs/terraform.tfstate"
    use_lockfile = true
    encrypt      = true
  }
}