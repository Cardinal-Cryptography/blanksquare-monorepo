# `@cardinal-cryptography/enclaves`

AWS Nitro Enclaves attestation verification module, providing comprehensive verification of attestation documents to ensure code is running in a genuine, trusted enclave environment.

## Overview

This package provides a complete solution for verifying AWS Nitro Enclaves attestation documents, including:

- **CBOR/COSE signature verification** - Validates cryptographic signatures using AWS root certificates
- **Certificate chain validation** - Ensures attestation documents are signed by legitimate AWS infrastructure
- **PCR measurement verification** - Validates Platform Configuration Register values against expected measurements
- **Attestation document parsing** - Extracts and validates all components of the attestation document

## Installation

```bash
npm install @cardinal-cryptography/enclaves
```

## License

Apache-2.0

## References

- [AWS Nitro Enclaves Attestation Process](https://github.com/aws/aws-nitro-enclaves-nsm-api/blob/main/docs/attestation_process.md)
- [CBOR Object Signing and Encryption (COSE)](https://tools.ietf.org/html/rfc8152)
- [AWS Nitro Enclaves Documentation](https://docs.aws.amazon.com/enclaves/)
