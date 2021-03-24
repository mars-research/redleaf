#!/usr/bin/env bash

BASE=$(realpath $(dirname "$0"))
IMAGE=${IMAGE:-zhaofengli/redleaf-dev:$(git rev-parse HEAD)}

sub_build() {
	set -e

	echo "Building $IMAGE..."
	artifact=$(nix-build $BASE/nix/docker-dev-image.nix --no-out-link)
	loaded=$(docker load < $artifact | sed -n 's/^Loaded image: \(.*\)/\1/p')
	docker tag $loaded $IMAGE
}

sub_shell() {
	echo "Starting Docker dev shell..."
	docker run --rm -it \
		-v $BASE:/redleaf -w /redleaf \
		-v $HOME:$HOME \
		-e HOME=$HOME \
		-e USER=$(id -nu) -e GROUP=$(id -ng) -e UID=$(id -u) -e GID=$(id -g) \
		$IMAGE passthrough-shell /bin/bash
}

sub_help() {
	echo "Usage: $0 <shell|build>"
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
