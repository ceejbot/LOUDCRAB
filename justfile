set dotenv-load := false

buildbox:
    docker build -t loudcross - < Dockerfile.builds

check-box:
    docker run -v $(PWD):/src -w /src --rm -it loudcross /bin/bash
