set dotenv-load

default:
  just --list

alias f := fmt
alias r := run
alias t := test

all: build test clippy fmt-check www-check

[group: 'misc']
build:
  cargo build --workspace

[group: 'check']
check:
 cargo check --workspace

[group: 'check']
ci: test clippy forbid
  cargo fmt --all -- --check
  cargo update --locked --package jobs-surf

[group: 'check']
clippy:
  cargo clippy --workspace --all-targets

[group: 'format']
fmt:
  cargo fmt

[group: 'format']
fmt-check:
  cargo fmt --all -- --check

[group: 'check']
forbid:
  ./bin/forbid

[group: 'misc']
install:
  cargo install -f jobs-surf

[group: 'dev']
install-dev-deps:
  cargo install cargo-watch

[group: 'release']
publish:
  ./bin/publish

[group: 'dev']
run *args:
  cargo run {{ args }}

[group: 'setup']
services:
  docker compose up --no-recreate -d

[group: 'setup']
stop-services:
  docker compose down

[group: 'test']
test:
  cargo test --workspace

[group: 'test']
test-release-workflow:
  -git tag -d test-release
  -git push origin :test-release
  git tag test-release
  git push origin test-release

[group: 'release']
update-changelog:
  echo >> CHANGELOG.md
  git log --pretty='format:- %s' >> CHANGELOG.md

[group: 'dev']
watch +COMMAND='test':
  cargo watch --clear --exec "{{ COMMAND }}"

[group: 'misc']
www-build:
  bun run --cwd www build

[group: 'check']
www-check:
  bun run --cwd www lint
  bun run --cwd www typecheck
  bun run --cwd www build

[group: 'dev']
www-dev:
  bun run --cwd www dev

[group: 'dev']
www-generate:
  cargo run --locked -- openapi --output openapi/jobs-surf.json
  bun run --cwd www generate:api

[group: 'setup']
www-install:
  bun install --cwd www --frozen-lockfile
