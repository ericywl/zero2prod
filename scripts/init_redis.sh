#!/usr/bin/env bash
set -x
set -eo pipefail

# Allow to skip Docker if a dockerized Redis is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
    RUNNING_CONTAINER=$(docker ps --filter 'name=zero2prod_redis' --format '{{.ID}}')
    if [[ -n $RUNNING_CONTAINER ]]; then
        echo >&2 "There is a DB container already running, kill it with"
        echo >&2 " docker kill zero2prod_redis && docker rm zero2prod_redis"
        exit 1
    fi

    docker run \
        --name "zero2prod_redis" \
        -p "6379:6379" \
        -d redis:7
fi

>&2 echo "Redis is ready to go!"