# EIF (Enclave Image File) Build Setup

This directory contains scripts and configuration for building EIF files from Docker images using AWS Nitro Enclaves CLI.

## ✅ The Solution

The Dockerfile has been reworked to follow the [official AWS installation guide](https://docs.aws.amazon.com/enclaves/latest/user/nitro-enclave-cli-install.html). We now use the official AWS packages in a Docker container:

```dockerfile
# Install official AWS Nitro Enclaves CLI packages
RUN dnf update -y && \
    dnf install -y \
    aws-nitro-enclaves-cli \
    aws-nitro-enclaves-cli-devel \
    docker \
    && dnf clean all
```

### Directory Structure

```text
├── .github/scripts/nitro-cli-docker/    # EIF build utilities
│   ├── Dockerfile.nitro-cli             # Docker image with nitro-cli
│   ├── build-entrypoint.sh              # Container entrypoint
│   └── build-enclave-image.sh           # Main build script
└── README.md                            # This file
```

## Build Methods

### 1. **Automated Docker Build** (Recommended)

```bash
cd eif/
chmod +x build-enclave-image.sh
./build-enclave-image.sh
```

### 2 **CI/CD Integration**

The project includes reusable GitHub Actions workflows:

- **`_build-eif-from-docker.yml`**: Reusable workflow for building EIFs
- **`publish-eif-release.yml`**: Creates GitHub releases with EIF artifacts

**Example workflow usage:**
```yaml
jobs:
  build-eif:
    uses: ./.github/workflows/_build-eif-from-docker.yml
    with:
      public-docker-image-url: "your-registry/image:tag"
      working-directory: "./crates/shielder-scheduler-tee/eif"
```

### Configuration

The build process is configured via environment variables:

**`ECR_IMAGE`** - The Docker image to convert:
```bash
# Default image
export ECR_IMAGE="public.ecr.aws/p5g6f1p8/shielder-scheduler:latest"

# Custom image
export ECR_IMAGE="your-registry/your-image:tag"
```

**`EIF_OUTPUT`** - The output filename for the EIF file:
```bash
# Default output filename
export EIF_OUTPUT="shielder-scheduler-tee.eif"

# Custom output filename
export EIF_OUTPUT="custom-enclave.eif"
```

Run the build:
```bash
./build-nitro-enclave-image.sh
```

### Output Files

After a successful build, you'll find:

- **`shielder-scheduler-tee.eif`** - The Enclave Image File ready for deployment
- **`eif-info.json`** - PCR values and attestation measurements