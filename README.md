# Overview

A Rust version of Andrej Karpathy's LLM-Council (not vibe coded)

## Requirements

This service calls the various front tier models (anthropic, gemini, gropk and openai). The service must be launched before starting the llm-council service.

It also depends on a document store service to store all stages of all responses.

**NB** The document server should be deployed on a separate server to where the unikernels are deployed.

The main reason is that the unikernels are deployed in a sandbox (complete isolation), the unikernel's file system is immutable and has no access to the host file system, whereas the document store needs access to the host's file system and as a result executes as a normal service and not a unikernel.

The only access they have is to connect to the hosts physical ethernet device.

Here are the depenedencies (links to repo's)

- [anthropic-service](https://github.com/lmzuccarelli/rust-ai-unikernel-anthropic-service)
- [gemini-service](https://github.com/lmzuccarelli/rust-ai-unikernel-gemini-service)
- [grok-service](https://github.com/lmzuccarelli/rust-ai-unikernel-grok-service)
- [openai-service](https://github.com/lmzuccarelli/rust-ai-unikernel-openai-service)
- [document-service](https://github.com/lmzuccarelli/rust-document-service)

## Usage

**NB** The unikernel launch process requires a statically linked elf binary, the following make recipe will build a static binary

Clone this repo

cd rust-ai-unikernel-llm-council

```
make fmt
make verify
make build
```

## Signing

The unikernel launch process checks to see if the binary has been signed.

To sign the binary use the "rust-microservice-package-manager" project.

Execute the following commands

Create a key-pair (ignore this step if you have already created a key-pair) for signing

```
./target/release/microservice-package-manager keypair
```

Sign the binary

```
./target/release/rust-microservice-package-manager sign --artifact <path-to-binary>
```

The signed artifact will be stored in the .ssh folder of rust-microservice-package-manager project 

## Configration

The config file should be left as is (dedecated for this application)

## Local Testing

Build as follows

```
make fmt
make verify
make build-local
```

Execute locally 

```
./target/release/ai-unikernel-llm-council
```
