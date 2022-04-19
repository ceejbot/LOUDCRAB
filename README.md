# LOUDCRAB

CRABS ARE VERY LOUD ANIMALS. NO, REALLY. THEY SCUTTLE AT TOP VOLUME.

LOUDBOT IS A SLACK BOT THAT SHOUTS AT YOU IF YOU SHOUT. IT REMEMBERS WHAT YOU SHOUT. IT ALSO RESPONDS TO SOME OTHER PROMPTS BUT IT WOULD SPOIL THE FUN TO DOCUMENT THEM. SHOUTING IS CATHARTIC. CEEJBOT ENJOYS WRITING LOUDBOT IN MANY PROGRAMMING LANGUAGES. IT'S A GOOD WAY TO LEARN TO USE LIBRARIES LIKE THE LOCAL REDIS LIBRARY, AND HOW TO DO BASIC TEXT PROCESSING.

## CONFIGURATION

Configuration is injected from environment variables. LOUDBOT will invoke read a `.env` file using dotenv at the start of its run, but you can use whatever daemon runner your OS uses to provide env vars.

A backing redis is required to remember what was shouted across runs. LOUDBOT will exit if it can't talk to a redis. Keys are prefixed with `LB:`.

Config vars:

- `SLACK_TOKEN`: Your slack api token. Required.
- `VERIFICATION_TOKEN`: The verification token in your app settings. Slack says this is deprecated, but it still sends it with every event it posts.
- `ROUTE_PREFIX` - an optional string to use to prefix LOUDBOT's two routes. Defaults to empty string.
- `REDIS_URL`: A URI giving the host:port of your redis. Defaults to `redis://localhost:6379`
- `WELCOME_CHANNEL`: The human name of the channel LOUDBOT should toast in when it starts up. Optional.
- `TUCKER_CHANCE`: The percentage chance [Malcolm Tucker](https://en.wikipedia.org/wiki/Malcolm_Tucker) will be invoked if you swear. Defaults to 2%. Malcolm only appears if certain four-letter words are used, so there is zero chance of sweary gifs in your Slack if you yourselves do not swear. Setting this to zero deactivates all Tucker appearances.
- `RUST_LOG`: One of `trace`, `debug`, `info`, `warn`, following [env_logger](https://lib.rs/crates/env_logger) convention.

## RUNNING

`cargo build` produces three executables: `LOUDBOT`, `SEED`, and `PRUNE`. Yes, they're all upper-case. We're here to __SHOUT__. You can also choose from the pre-built releases here on GitHub.

1. LOUDBOT uses the Slack events API, so it needs to be listening on a publically-accessible address somewhere that Slack can post events to. Yes this is a pain. Slack deprecated its RTM api so, you know, here we are.
2. Set up a Redis accessible to its run environment.
3. Create an application in Slack. Give it a bot user using the modern "granular" permissions. Take note of the verification token; this is `VERIFICATION_TOKEN`. [This Slack docs page might help](https://api.slack.com/bot-users).
4. LOUDBOT needs these permissions: `chat:write`, `chat:write:customize`, `emoji:read`, `reactions:read`, `reactions:write`.
5. Install the app into your Slack team. Take note of the bot user access token; this is `SLACK_TOKEN`.
6. Provide configuration via environment variables.
7. Run `SEED`. It takes an optional list of file paths, which must be newline-delimited text files. It stores each line as a shout in your backing redis. If you have no seeds, why not use the provided classic set in [`SEEDS`](https://github.com/ceejbot/LOUDCRAB/blob/latest/SEEDS)? You don't need to run this to have a functional `LOUDBOT`, but some easter eggs will not function without seeding data into redis.
8. Run `LOUDBOT` as a daemon where it has access to the redis.  If you gave it a toast channel, a working LOUDBOT will toast you now. No toast? Double-check your auth token.
9. Back on the Slack website, add __Event Subscriptions__ as a feature for your app. The request url should be `/incoming` plus whatever route prefix you set up (if indeed you need a prefix). This step needs to be last because Slack will immediately post a challenge to the URL and will not send events until the app responds.
10. Subscribe to these bot events: `app_mention`, `message.channels` and `reaction_added`.
11. Invite the LOUDBOT bot user to a channel. SHOUT WHERE LOUDBOT CAN HEAR. IT SHOULD SHOUT BACK.

Yes, this is all much more annoying than it used to be. RTM was easier to cope with.

LOUDBOT also responds to `GET /monitor/ping` with a random shout. This is a useful liveness check.

## MANAGING

ARE YOU UPSET BY WHAT LOUDBOT SHOUTS? LOUDBOT IS YOU.

But sometimes we wish to forget. `PRUNE` is an administrative convenience for making LOUDBOT bulk-forget shouts. Put the items you'd like to purge as new-line delimited text in some file, then run `PRUNE /path/to/file`.

## BUILDING

There's a [justfile](https://github.com/casey/just) that will build a tarred-up release for your architecture if it's not covered by the prebuilts. `cargo doc --open` will show you internal maintainer docs.

## TODO

- Switch to validating request signatures from Slack instead of using the verification token.
- Loudie lost the ability to add emoji reactions a couple of Slack API deprecations ago. It would be great to restore it.
- Better backup/restore for shouts.
- Maybe a better administration tool for removing unwelcome shouts.
- Clean up the github workflow.
- Migrate to whatever the heck this week's Slack API is with [socket mode](https://api.slack.com/apis/connections).

## License

MIT.
