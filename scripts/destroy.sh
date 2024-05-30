#!/usr/bin/env bash
set -x
set -eo pipefail

docker kill zero2prod_db
docker rm zero2prod_db

docker kill zero2prod_redis
docker rm zero2prod_redis