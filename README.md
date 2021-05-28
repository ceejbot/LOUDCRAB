# LOUDCRAB

CRABS ARE VERY LOUD ANIMALS. NO, REALLY. THEY SCUTTLE AT TOP VOLUME.

LOUDBOT IS A SLACK BOT THAT SHOUTS AT YOU IF YOU SHOUT. IT ALSO RESPONDS TO SOME OTHER PROMPTS BUT IT WOULD SPOIL THE FUN TO DOCUMENT THEM. SHOUTING IS CATHARTIC. CEEJBOT ENJOYS WRITING LOUDBOT IN MANY PROGRAMMING LANGUAGES. IT'S A GOOD WAY TO LEARN TO USE LIBRARIES LIKE THE LOCAL REDIS LIBRARY, AND HOW TO DO BASIC TEXT PROCESSING.

## CONFIGURATION

Configuration is injected from environment variables. LOUDBOT will invoke read a `.env` file using dotenv at the start of its run, but you can use whatever daemon runner your OS uses to provide env vars.

A backing redis is required to remember what was shouted across runs. LOUDBOT will exit if it can't talk to a redis. Keys are prefixed with `LB:`.

Config vars:

- `SLACK_TOKEN`: Your slack api token. Required.
- `VERIFICATION_TOKEN`: The verification token in your app settings. Slack says this is deprecated, but it still sends it with every event it posts.
- `ROUTE_PREFIX` - an optional string to use to prefix LOUDBOT's two routes. Defaults to empty string.
- `REDIS_URL`: A URI giving the host:port of your redis. Defaults to `redis://localhost:6379`
- `WELCOME_CHANNEL`: The human name of the channel LOUDBOT should toast in when it starts up. Optional.
- `TUCKER_CHANCE`: The percentage chance Malcolm Tucker will be invoked if you swear. Defaults to 2%. Malcolm only appears if certain four-letter words are used, so there is zero chance of sweary gifs in your Slack if you yourselves do not swear.
- `RUST_LOG`: One of `trace`, `debug`, `info`, `warn`, following [env_logger](https://lib.rs/crates/env_logger) convention.

## RUNNING

`cargo build` produces three executables: `LOUDBOT`, `SEED`, and `PRUNE`. Yes, they're all upper-case. We're here to __SHOUT__. You can also choose from the pre-built releases here on GitHub.

1. LOUDBOT uses the Slack events API, so it needs to be listening on a publically-accessible address somewhere that Slack can post events to.
2. Set up a Redis accessible to its run environment.
3. Create an application in Slack. Give it a bot user using the modern "granular" permissions. Take note of the verification token; this is `VERIFICATION_TOKEN`. [This Slack docs page might help](https://api.slack.com/bot-users).
4. LOUDBOT needs these permissions: `chat:write`, `chat:write:customize`, `emoji:read`, `reactions:read`, `reactions:write`.
5. Install the app into your Slack team. Take note of the bot user access token; this is `SLACK_TOKEN`.
6. Provide configuration via environment variables.
7. Run `SEED`. It takes an optional list of file paths, which must be newline-delimited text files. It stores each line as a shout in your backing redis. If you have no seeds, why not use the provided classic set in [`SEEDS`](https://github.com/ceejbot/LOUDCRAB/blob/latest/SEEDS)?
8. Run `LOUDBOT` as a daemon where it has access to the redis. You don't need the seed text files to run `LOUDBOT`. If you gave it a toast channel, a working LOUDBOT will toast you now.
9. Add __Event Subscriptions__ as a feature for your app. The request url should be `/incoming` plus whatever route prefix you set up (if indeed you need a prefix). This step needs to be last because Slack will immediately post a challenge to the URL and will not send events until the app responds.
10. Subscribe to these bot events: `app_mention`, `message.channels` and `reaction_added`.
11. Invite the LOUDBOT bot user to a channel. SHOUT WHERE LOUDBOT CAN HEAR. IT SHOULD SHOUT BACK.

Yes, this is all much more annoying than it used to be. RTM was easier to cope with.

## MANAGING

ARE YOU UPSET BY WHAT LOUDBOT SHOUTS? LOUDBOT IS YOU.

But sometimes we wish to forget. `PRUNE` is an administrative convenience for making LOUDBOT bulk-forget shouts. Put the items you'd like to purge as new-line delimited text in some file, then run `PRUNE /path/to/file`. I need something better here myself so I'll write it soon. I also need a good backup method better than redis dumps.

## RUNNING WITH DOCKER

There's a Dockerfile that builds a reasonably tiny alpine run environment for a LOUDBOT. The tradeoff is that you'll need to run the [Rust embedded cross-compilation tools](https://github.com/rust-embedded/cross) yourself to get executables. This is mostly painless: type `make alpine`. `make all` builds darwin, linux gnu, and linux musl (for alpine) targets for you, tars them up, and stashes them in `./releases`.

The example Docker file takes the required config as build arguments; be careful with that slack api token.

## License

MIT.
