# testing

0. make sure the app can accept requests.
  - if you have access to a remote host, save the script below to `dev/tunnel.sh` and use it to forward traffic to the app run locally.
  - otherwise, use something like https://ngrok.com/ which would do that for you.
1. [register](https://github.com/settings/apps/new) a new GitHub app, then add read/write access to pull requests and issues.
2. open its GitHub Store page[^1] and install it on a selected repository.
3. list of events sent to the app is available on `https://github.com/settings/apps/{app name}/advanced`.

## nginx setup

see `dev/example.nginx` to avoid being a web framework canary

## port forwarding

(this can probably be boiled down to just a single `ssh` command, but I'm not very good at juggling `-L`s and `-R`s)

```sh
#!/usr/bin/env bash

# -R used  locally: on jump_host, forward traffic from ITS localhost:jump_local_port to YOUR localhost:local_port
# -L used remotely: accept traffic from any ip (0.0.0.0) on external_port and forward it to localhost:local_jump_port

# to sum it up: anyone → jump_host:external_port → jump_host:local_jump_port → localhost:local_port → local web server

external_port="8000"
jump_local_port="12345"
local_port="3000"
jump_user="user"
jump_host="domain-or-ip.com"

ssh -R "${jump_local_port}":localhost:"${local_port}" "${jump_user}@${jump_host}" -A \
    "ssh -L 0.0.0.0:${external_port}:localhost:${jump_local_port} ${jump_user}@localhost >/dev/null"
```

[^1]: `https://github.com/apps/{app name}`
