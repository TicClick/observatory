#!/bin/bash

full_repo_name="TicClick/observatory"
releases_url="https://api.github.com/repos/${full_repo_name}/releases/latest"
package_url=$( curl "${releases_url}" | jq '.assets | map(select(.name | endswith("linux-gnu.tar.gz"))) | .[0].browser_download_url' --raw-output )
if [[ "${package_url}" = "null" ]]; then
    echo "no release available -- see https://github.com/${full_repo_name}/releases/latest" && exit 1
fi

# extract everything into current directory, assuming there's only an executable inside (otherwise there'll be A LOT of litter)
curl -L "$package_url" | tar -zxf -
