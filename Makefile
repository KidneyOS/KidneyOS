.PHONY: build
build:
	cargo build -Zbuild-std=core --target targets/i686-unknown-kernel.json

.PHONY: test
test:
	cargo test --target i686-unknown-linux-gnu

.PHONY: clean
clean:
	cargo clean
