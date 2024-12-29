# DeepSeek Bot

This is a Telegram bot server and wrapper for [DeepSeek API](https://api-docs.deepseek.com/).

# Configuration

```toml
# config.toml to be put under repository root
telegram_bot_token = "..."  # bot token from t.me/botfather
deepseek_api_token = "..."  # DeepSeek api token from platform.deepseek.com
```

```toml
# trustedusers.toml to be put under repository root
trusted_users = [...]  # a list of strings, representing trusted users' uids
```

The bot supports inline queries. You may need to enable this feature in your bot configuration at [@botfather](https://t.me/botfather).

# Build

This repository contains a `.envrc` file. If you have `nix` and `direnv` installed,
entering the directory will let `direnv` automatically deploy the development environment for you.

```sh
cargo run
```
