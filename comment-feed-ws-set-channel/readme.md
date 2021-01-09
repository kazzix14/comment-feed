https://github.com/awslabs/aws-lambda-rust-runtime

# build and package deploy-ready artifact

```sh
docker run --rm \
    -v ${PWD}:/code \
    -v ${HOME}/.cargo/registry:/root/.cargo/registry \
    -v ${HOME}/.cargo/git:/root/.cargo/git \
    softprops/lambda-rust
```

# start a docker container replicating the "provided" lambda runtime

# awaiting an event to be provided via stdin

```sh
unzip -o \
    target/lambda/release/bootstrap.zip \
    -d /tmp/lambda && \
  docker run \
    -i -e DOCKER_LAMBDA_USE_STDIN=1 \
    --rm \
    -v /tmp/lambda:/var/task \
    lambci/lambda:provided
```

# provide an event payload via stdin (typically a json blob)

# Ctrl-D to yield control back to your function
