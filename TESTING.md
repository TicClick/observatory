# testing

## github app setup

1. [register](https://github.com/settings/apps/new) a new GitHub app with read/write access to pull requests and issues
2. set webhook URL to your server endpoint (see deployment section below)
3. install the app on a selected repository via `https://github.com/apps/{app name}`
4. webhook events are available at `https://github.com/settings/apps/{app name}/advanced`

## local development

### with docker

```shell
docker compose up
```

app runs at `http://localhost:3000`

### without docker

```shell
cargo run -- -c runtime/config.yaml
```

## unit tests

```shell
cargo test
cargo tarpaulin --out html  # coverage report
```

## deployment

automated via GitHub Actions:
- **release**: push a tag (e.g., `v1.2.3`) to build and publish docker image to ghcr.io
- **deploy**: manually trigger from GitHub Actions UI to deploy specific image tag to server

see nginx config example in `dev/example.nginx`
