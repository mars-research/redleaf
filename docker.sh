#!/usr/bin/env bash

set -e

BASE=$(dirname "$0")
IMAGE=${IMAGE:-zhaofengli/redleaf-dev:$(git rev-parse HEAD)}

sub_build() {
	echo "Building $IMAGE..."
	artifact=$(nix-build $BASE/nix/docker-dev-image.nix --no-out-link)
	docker load $IMAGE < $artifact
}

sub_shell() {
	echo "Starting Docker dev shell..."
	docker run --rm -it \
		-v $PWD:/redleaf -w /redleaf \
		-v $HOME:$HOME \
		-e HOME=$HOME \
		-e USER=$(id -nu) -e GROUP=$(id -ng) -e UID=$(id -u) -e GID=$(id -g) \
		$IMAGE passthrough-shell /bin/bash
}

sub_help() {
	true
}

subcommand=$1
case $subcommand in
	"")
		sub_shell
		;;
    "-h" | "--help")
        sub_help
        ;;
    *)
        shift
        sub_${subcommand} $@
        if [ $? = 127 ]; then
            echo "Error: '$subcommand' is not a known subcommand." >&2
            echo "       Run '$ProgName --help' for a list of known subcommands." >&2
            exit 1
        fi
        ;;
esac
