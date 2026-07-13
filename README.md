# pg_punycode

[![CI](https://github.com/sindrip/pg_punycode/actions/workflows/ci.yml/badge.svg)](https://github.com/sindrip/pg_punycode/actions/workflows/ci.yml)
[![PostgreSQL 13+](https://img.shields.io/badge/PostgreSQL-13+-blue)](#development)
[![Release](https://img.shields.io/github/v/release/sindrip/pg_punycode)](https://github.com/sindrip/pg_punycode/releases)
[![License](https://img.shields.io/github/license/sindrip/pg_punycode)](LICENSE)

Punycode (RFC 3492) and IDNA/UTS-46 domain name functions for PostgreSQL,
written in Rust with [pgrx](https://github.com/pgcentralfoundation/pgrx) and
backed by the [idna](https://crates.io/crates/idna) crate (the rust-url /
Firefox implementation).

```sql
CREATE EXTENSION pg_punycode;

SELECT domain_to_ascii('Bücher.DE');                  -- xn--bcher-kva.de
SELECT domain_to_unicode('xn--fiqs8s');               -- 中国
SELECT punycode_encode('bücher');                     -- bcher-kva
SELECT punycode_decode('MajiKoi5-783gue6qz075azm5e'); -- MajiでKoiする5秒前
```

## Overview

Encoding happens in the database, so every client sees the same canonical
form — no per-application punycode libraries to keep in sync. All functions
are `STRICT`, `IMMUTABLE`, and `PARALLEL SAFE`, so they can back indexes and
constraints:

```sql
CREATE INDEX domains_ace_idx ON domains (domain_to_ascii(name));
```

## Functions

Invalid input raises an error.

| Function | Description |
| --- | --- |
| `punycode_encode(text) → text` | Raw RFC 3492 Punycode over a single label: no `xn--` prefix, no dot-splitting. Pure-ASCII input gets a trailing `-` per the RFC. |
| `punycode_decode(text) → text` | Inverse of `punycode_encode`; extended digits are case-insensitive. |
| `domain_to_ascii(text) → text` | IDNA A-label ("xn--") form of a domain name per UTS-46 with WHATWG URL rules: case-mapped, normalized, validated. |
| `domain_to_unicode(text) → text` | IDNA U-label (Unicode) form of a domain name per UTS-46. |

## Installation

### From release artifacts

Each [release](https://github.com/sindrip/pg_punycode/releases) ships
per-major tarballs for Linux (amd64 and arm64). Unpack and copy into your
server's directories:

```sh
tar -xzf pg_punycode-v*-pg18-linux-amd64.tar.gz
cd pg_punycode-v*-pg18-linux-amd64
sudo cp lib/* "$(pg_config --pkglibdir)"
sudo cp share/extension/* "$(pg_config --sharedir)/extension"
```

### From source

Requires Rust (rustup picks the toolchain from `rust-toolchain.toml`) and a
PostgreSQL installation with headers (`pg_config` on PATH):

```sh
cd crates/pg_punycode
cargo xtask pgrx -- install --release --pg-config "$(command -v pg_config)"
```

The matching `cargo-pgrx` CLI is bootstrapped automatically into `.tools/`
from the workspace's pinned `pgrx` version — no global installs.

## Development

Everything runs through `cargo xtask`, which keeps the cargo-pgrx CLI in
lockstep with the `pgrx` crate pin:

```sh
cargo xtask pgrx -- init --pg18 download   # one-time: dev Postgres into ~/.pgrx
cargo xtask test pg18                      # #[pg_test] suite
cargo xtask run pg18                       # build + install + psql
cd crates/pg_punycode && cargo xtask pgrx -- regress pg18   # pg_regress suite
```

Supported PostgreSQL majors: 13–19 (19 is beta), all exercised in CI.

## Releases

Releasing is merging a version bump: change `[workspace.package] version`
(it flows into the extension's `default_version`) and push to `main`. Once
CI passes, the release workflow notices the unreleased version and builds a
draft release with per-major Linux tarballs (amd64/arm64); the release and
its `v<version>` tag go live only once every artifact is attached. Pushes
that don't change the version release nothing.

## License

[MIT](LICENSE)
