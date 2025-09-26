## Overview

This directory contains Rust crates required to build Shielder Prover Server, a TEE-based REST server which
computes ZK-proofs:
1. The user wants to generate a proof for relation `R` with witness `w` and statement `s`
2. The user generates an asymmetric encryption key (`pub_sk`, `sk`) to get a response back from the server
3. The user queries server for its public key `pub_tee` and attestation document
4. The user validates attestation document, to prove TEE server identity.
5. The user encrypts `pub_tee(R, w, s, pub_sk)`, and sends encrypted blob to the server
6. The server decrypts the message using its private key. Then it generates the proof `π` and `pub_inputs`, and outputs `pub_sk(π)`, `pub_sk(pub_inputs)`
7. The user receives encrypted (`π`, `pub_inputs`) and decrypts it.

The proof `π` and `pub_inputs` can be then used in further part of the Shielder workflow, ie submitting the proof to the Shielder contract.

### Packages

There are three Rust crates located in the main workspace under `crates/`:
* `crates/shielder-prover-common` - contains common definitions between the `shielder-prover-server` and `shielder-prover-tee`,
* `crates/shielder-prover-server` - a host (EC-2) part of the server. This is the server that is exposed to the Internet, and most
of its function is to forward requests to TEE and limit maximum concurrent requests amount
* `crates/shielder-prover-tee` - main part of the Server which computes ZK-proofs, runs entirerely in TEE. Communicates with
`shielder-prover-server` via vsock

## Building

The enclave image of `shielder-prover-tee` is built using dockerized nitro-cli as part of the CI pipeline. The build process:

1. **Docker Image**: First, a Docker image is built from `crates/shielder-prover-server/docker/Dockerfile`
2. **EIF Generation**: The Docker image is then converted to an Enclave Image File (EIF) using AWS nitro-cli in a Docker container

### Manual Build (for development)

To manually build the Docker image:
```bash
cd crates/shielder-prover-server
docker build -f docker/Dockerfile -t shielder-prover:local .
```

The EIF generation is handled by the CI pipeline using the `_build-eif-from-docker.yml` workflow, which uses dockerized nitro-cli for reproducible builds.

### Local testing

`shielder-prover-tee` requires AWS Nitro environment to be run, in particular to perform [attestation](https://github.com/aws/aws-nitro-enclaves-nsm-api/blob/main/docs/attestation_process.md)
using NSM driver. Although this driver is [included](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=b9873755a6c8ccfce79094c4dce9efa3ecb1a749) 
in Linux kernel 6.8+, it might be missing on your local dev environment. To run `shielder-prover-tee` without attestation, run:
```bash
RUST_LOG=info cargo run --release -p shielder-prover-tee --features without_attestation
```

