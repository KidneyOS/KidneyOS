.PHONY: build
build:
	cargo rustc \
	  --bin kidney-os \
	  --manifest-path Cargo.toml \
	  --target targets/i686-unknown-kernel.json \
	  --release \
	  -Z build-std=core \
	  -- \
	  -C link-arg=-T -C link-arg=linkers/i686.ld \
	  -C link-arg=-z -C link-arg=max-page-size=0x1000 \
	  -C link-arg=-S \
	  --emit link=kernel

.PHONY: test
test:
	cargo test --target i686-unknown-linux-gnu

.PHONY: clean
clean:
	cargo clean
	rm -f kernel
