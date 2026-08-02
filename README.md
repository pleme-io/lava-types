# lava-types

Typed constraint validators for lava architectures — the `Dry::Struct` analog
for the [lava](https://github.com/pleme-io/lava-core) suite.

Constraints are validated at **composition time, not apply time**: an invalid
CIDR fails the plan, never the cloud API. That moves a whole class of
infrastructure error from a slow, partially-applied runtime failure to a fast,
local one.

## Install

```toml
[dependencies]
lava-types = "0.1"
```

## Validators

`cidr` · `port` · `protocol` · `enum` · `regex` · `length` · `range` ·
`ipv4` · `ipv6` · `hostname`

In tatara-lisp these are authored as constraint forms:

```lisp
(:type :cidr-block)
```

which is the lava equivalent of Pangea's
`Types::String.constrained(...)`.

## The suite

`lava-types` is a leaf — it depends on no other lava crate, and
[`lava-schema`](https://github.com/pleme-io/lava-schema) builds on it.

## License

MIT
