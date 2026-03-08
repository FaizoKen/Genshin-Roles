# Genshin Player Role

A [RoleLogic](https://rolelogic.faizo.net) plugin that links Discord accounts with Genshin Impact player data via [Enka.Network](https://enka.network). Users verify UID ownership by placing a code in their in-game signature, then roles are automatically assigned based on player progress (AR level, Spiral Abyss, achievements, etc.).

## How it works

1. **Registers** guild/role pairs via the RoleLogic plugin API
2. **Authenticates** users through Discord OAuth
3. **Verifies** Genshin UID ownership by checking a generated code in the player's in-game signature
4. **Fetches** player data from Enka.Network (characters, Spiral Abyss, achievements)
5. **Syncs** verified player data to RoleLogic for automatic role assignment based on configurable conditions

## Setup

```bash
cp .env.example .env
# Edit .env with your values
```

### Environment Variables

| Variable                | Required | Default                              | Description                     |
| ----------------------- | -------- | ------------------------------------ | ------------------------------- |
| `DATABASE_URL`          | Yes      | —                                    | PostgreSQL connection string    |
| `DISCORD_CLIENT_ID`     | Yes      | —                                    | Discord OAuth app ID            |
| `DISCORD_CLIENT_SECRET` | Yes      | —                                    | Discord OAuth app secret        |
| `SESSION_SECRET`        | Yes      | —                                    | Secret for session encryption   |
| `BASE_URL`              | Yes      | —                                    | Public URL for redirects        |
| `LISTEN_ADDR`           | No       | `0.0.0.0:8080`                       | Server bind address             |
| `ENKA_USER_AGENT`       | No       | `GenshinRoles/1.0`                   | User agent for Enka.Network API |
| `RUST_LOG`              | No       | `genshin_roles=info,tower_http=info` | Log level                       |

## Run

### Docker (recommended)

```bash
docker compose up -d
```

### From source

```bash
cargo run              # development
cargo build --release  # production
```

## Endpoints

| Method   | Path                       | Description                             |
| -------- | -------------------------- | --------------------------------------- |
| `GET`    | `/health`                  | Health check                            |
| `POST`   | `/register`                | Register a guild/role pair              |
| `GET`    | `/config`                  | Get plugin configuration                |
| `POST`   | `/config`                  | Update role link conditions             |
| `DELETE` | `/config`                  | Delete a registration                   |
| `GET`    | `/verify`                  | Verification page                       |
| `GET`    | `/verify/login`            | Discord OAuth login                     |
| `GET`    | `/verify/callback`         | Discord OAuth callback                  |
| `GET`    | `/verify/status`           | Check linked account status             |
| `POST`   | `/verify/start`            | Start verification with a Genshin UID   |
| `POST`   | `/verify/check`            | Check verification code in signature    |
| `POST`   | `/verify/unlink`           | Unlink Discord account from Genshin UID |
| `GET`    | `/players/{guild_id}`      | Player list page                        |
| `GET`    | `/players/{guild_id}/data` | Paginated player data (JSON)            |

## Usage

1. In the RoleLogic dashboard, create a Role Link and set the **Custom Plugin URL** to your instance's public URL
2. RoleLogic will automatically register the guild/role pair
3. Users visit the verification page, sign in with Discord, and link their Genshin UID
4. Roles are assigned automatically based on the conditions you configure

## API Reference

- [RoleLogic Role Link API](https://docs-rolelogic.faizo.net/reference/role-link-api)
- [Enka.Network API](https://api.enka.network)

## License

[MIT](LICENSE)
