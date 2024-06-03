.PHONY: install
install:
	cargo install cargo-tarpaulin
	cargo install cargo-udeps

.PHONY: clean
clean:
	rm -rf .fake_emails/

.PHONY: run
run:
	cargo run | bunyan

.PHONY: up
up:
	./scripts/init_db.sh
	./scripts/init_redis.sh

.PHONY: down
down:
	./scripts/destroy.sh

.PHONY: test
test:
	cargo test

.PHONY: lint
lint:
	cargo clippy

.PHONY: coverage
coverage:
	cargo tarpaulin --ignore-tests

.PHONY: watch
watch:
	cargo watch -x check -x test -x run | bunyan