FROM alpine:latest

LABEL maintainer="C J Silverio <ceejceej@gmail.com>"

ARG redis_url=redis://127.0.0.1:6379
ARG slack_token=0xdeadbeef
ARG welcome
ARG tucker_chance=2
ARG log_level=info

ENV REDIS_URL=$redis_url
ENV SLACK_TOKEN=$slack_token
ENV WELCOME_CHANNEL=$welcome
ENV TUCKER_CHANCE=$tucker_chance
ENV RUST_LOG=$log_level

RUN apk add --no-cache bash libc6-compat
WORKDIR /loudbot
ADD ./built/loudbot-unknown-linux-musl.tgz .

CMD ["/loudbot/LOUDBOT"]
