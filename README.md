# LOUDCRAB

GO LOUD OR GO HOME. LOUDBOT IS A SLACK BOT THAT SHOUTS AT YOU IF YOU SHOUT AT IT. SHOUTING IS CATHARTIC. CEEJBOT ENJOYS WRITING LOUDBOT IN MANY PROGRAMMING LANGUAGES. IT'S A GOOD WAY TO LEARN TO USE LIBRARIES LIKE THE LOCAL REDIS LIBRARY, AND HOW TO DO BASIC TEXT PROCESSING.

Configuration is injected from environment variables. A backing redis is required to remember what was shouted across runs.

`cargo build` produces three executables: `LOUDBOT`, `SEED`, and `PRUNE`. Yes, they're all upper-case. We're here to __SHOUT__.

Run `SEED` first. It will read the contents of the seed text files and store shouts in your backing redis.

Run `LOUDBOT` as a daemon where it has access to the redis. You don't need the seed text files to run `LOUDBOT`.

`PRUNE` is an administrative convenience for making LOUDBOT bulk-forget shouts. Put the items you'd like to purge as new-line delimited text in the file `PRUNES`, then run `PRUNE`.

## RUNNING

1. Create an application in Slack. Name it LOUDBOT or something similar.
2. Add a bot user to your app and snag its API token. Invite the bot user to a couple of channels
3. Set up a redis somewhere.
4. Provide configuration via environment variables somehow. Create an env file or copy the example: `cp env.example .env`. Edit it so it points to your redis and uses your slack token.
5. Something.

## Configuration

- `SLACK_TOKEN`: Your slack api token. Required.
- `REDIS_URL`: host:port of your redis, defaults to `localhost:6379`
- `REDIS_PREFIX`: String prefix to use to namespace redis keys. Defaults to `LB`.
- `WELCOME_CHANNEL`: The human name of the channel LOUDBOT should toast in when it starts up. Optional.

## License

MIT.
