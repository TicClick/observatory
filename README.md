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

## monitoring

the simplest option is healthchecks.io -- create a check with 5m period + 10m grace, then add a cron job with `crontab -e`:

```
*/5 * * * * curl -fsS -m 10 -o /dev/null http://127.0.0.1:3000/; curl -fsS -m 10 --retry 3 -o /dev/null https://hc-ping.com/<UUID>/$?
```
