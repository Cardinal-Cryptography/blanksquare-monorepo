# Shielder Scheduler TEE Nix Build

This directory contains Nix configuration for building AWS Nitro Enclave EIF (Enclave Image Format) files for the shielder-scheduler-tee.

## Prerequisites

1. [Nix](https://nixos.org/download.html) with flakes enabled
2. Access to the ECR registry containing the shielder-scheduler-tee Docker image
3. Docker or skopeo for image extraction

## Usage

### 1. Set ECR image (optional)

By default, the build will use a default ECR image. You can override this:

```bash
export ECR_IMAGE="public.ecr.aws/p5g6f1p8/shielder-scheduler:latest"
```

### 2. Build the EIF file

```bash
# From this directory (crates/shielder-scheduler-tee/nix)
nix build

# The EIF file will be available in the result symlink
ls -la result/
```

### 3. Using specific image versions

```bash
# Use a specific tag
ECR_IMAGE="public.ecr.aws/p5g6f1p8/shielder-scheduler:latest:v1.2.3" nix build
```

## How it works

1. The Nix build pulls the specified Docker image from ECR using `dockerTools.pullImage`
2. Extracts the complete Docker image filesystem (not just the binary)
3. Uses `nitro-util.buildEif` with the Docker image filesystem as `copyToRoot`
4. Creates an AWS Nitro Enclave EIF file that uses the entire Docker image as the enclave root

This approach matches `nitro-cli --docker-uri` behavior, where the complete Docker image becomes the enclave root filesystem.

## Output

The build will produce an EIF file suitable for running in AWS Nitro Enclaves. The EIF contains the complete Docker image filesystem as the enclave root.

- Output file: `shielder-scheduler-tee-eif-x86_64`
- Architecture: x86_64 
- Root filesystem: Complete Docker image from ECR
- Entrypoint: `/usr/local/bin/shielder-scheduler-tee` (configurable in `enclave.nix`)

## Configuration

Update the `defaultImage` variable in `flake.nix` to point to your actual ECR registry:

```nix
defaultImage = "public.ecr.aws/your-actual-registry/shielder-scheduler:latest";
```

## Directory Structure

- `flake.nix` - Main Nix flake configuration
- `enclave.nix` - Enclave-specific build configuration  
- `flake.lock` - Locked dependency versions (auto-generated)
