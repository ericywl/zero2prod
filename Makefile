.PHONY: clean
clean:
	rm -rf .fake_emails/

.PHONY: run
run: up
	cargo run | bunyan

.PHONY: up
up:
	./scripts/init_db.sh

.PHONY: down
down:
	./scripts/destroy_db.sh

.PHONY: test
test:
	cargo test

.PHONY: lint
lint:
	cargo clippy