# LOUDCRAB

CRABS ARE VERY LOUD ANIMALS. NO, REALLY. THEY SCUTTLE AT TOP VOLUME!

LOUDBOT IS A SLACK BOT THAT SHOUTS AT YOU IF YOU SHOUT. IT ALSO RESPONDS TO SOME OTHER PROMPTS BUT IT WOULD SPOIL THE FUN TO DOCUMENT THEM. SHOUTING IS CATHARTIC. CEEJBOT ENJOYS WRITING LOUDBOT IN MANY PROGRAMMING LANGUAGES. IT'S A GOOD WAY TO LEARN TO USE LIBRARIES LIKE THE LOCAL REDIS LIBRARY, AND HOW TO DO BASIC TEXT PROCESSING.

## CONFIGURATION

Configuration is injected from environment variables. LOUDBOT will invoke read a `.env` file at the start of its run, but you can use whatever daemon runner your OS uses to provide env vars.

A backing redis is required to remember what was shouted across runs. LOUDBOT will exit if it can't talk to a redis. Keys are prefixed with `LB:`.

Config vars:

- `SLACK_TOKEN`: Your slack api token. Required.
- `REDIS_URL`: A URI giving the host:port of your redis. Defaults to `redis://localhost:6379`
- `WELCOME_CHANNEL`: The human name of the channel LOUDBOT should toast in when it starts up. Optional.
- `TUCKER_CHANCE`: The percentage chance Malcolm Tucker will be invoked if you swear. Malcolm only appears if certain four-letter words are used, so there is zero chance of sweary gifs in your Slack if you yourselves do not swear.

## RUNNING

`cargo build` produces three executables: `LOUDBOT`, `SEED`, and `PRUNE`. Yes, they're all upper-case. We're here to __SHOUT__. You can also choose from the pre-built releases here on GitHub.

1. Create an application in Slack. Name it LOUDBOT or something similar.
2. Add a bot user to your app and snag its API token. Invite the bot user to a couple of channels.
3. Set up a redis somewhere.
4. Provide configuration via environment variables somehow.
5. Run `SEED`. It takes an optional list of file paths, which must be newline-delimited text files. It stores each line as a shout in your backing redis. If you have no seeds, why not use the provided classic set in [`SEEDS`](https://github.com/ceejbot/LOUDCRAB/blob/latest/SEEDS)?
6. Run `LOUDBOT` as a daemon where it has access to the redis. You don't need the seed text files to run `LOUDBOT`.

If you gave it a toast channel, you can now see some toasts.

`PRUNE` is an administrative convenience for making LOUDBOT bulk-forget shouts. Put the items you'd like to purge as new-line delimited text in some file, then run `PRUNE /path/to/file`. I need something better here myself so I'll write it soon. I also need a good backup method better than redis dumps.

## Running with Docker

There's a Dockerfile that builds a reasonably tiny alpine run environment for a LOUDBOT. The tradeoff is that you'll need to run the [Rust embedded cross-compilation tools](https://github.com/rust-embedded/cross) yourself to get executables. This is mostly painless: type `make alpine`. `make all` builds darwin, linux gnu, and linux musl (for alpine) targets for you, tars them up, and stashes them in `./releases`.

The example Docker file takes the required config as build arguments; be careful with that slack api token.

## License

MIT.
