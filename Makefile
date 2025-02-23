.PHONY: check clean doc doc-std run test
DB=uploads.db

.env:
	sed 's/\/var\/lib\/uploads\///' deb/env > $@

include .env
export

check: .git/hooks/pre-commit
	. $<

clean:
	rm -f uploads.db* .env

doc:
	cargo doc --open

doc-std:
	rustup doc --std

run: $(DB)
	cargo sqlx prepare
	cargo run

test:
	cargo test

$(DB):
	cargo sqlx database create && cargo sqlx migrate run

.git/hooks/pre-commit:
	curl -o $@ https://gist.githubusercontent.com/paasim/317a1fd91a6236ca36d1c1c00c2a02d5/raw/767f2ab0b59e6bf5fe5c44608a872c5293f6e64e/rust-pre-commit.sh
	echo "" >> $@
	chmod +x $@
