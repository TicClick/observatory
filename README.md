# observatory

GitHub app for detecting overlapping translation changes in [osu! wiki](https://github.com/ppy/osu-wiki)

## features

- detect overlapping changes (same `.md` files edited)
- detect original change and a translation existing at the same time

## quick start

```shell
# local development
docker compose up

# or run directly
cargo run -- -c runtime/config.yaml
```

## testing

see [`TESTING.md`](TESTING.md) for details

## deploying

- **release**: `git tag v1.2.3 && git push origin v1.2.3` builds and publishes to ghcr.io
- **deploy**: trigger manually from GitHub Actions with desired image tag
