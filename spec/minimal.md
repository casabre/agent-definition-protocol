# ADP Minimal Specification v0.1.0

ADP-Minimal defines the smallest valid agent manifest. It is useful for quick wiring or composition before full details are attached.

## Conformance language
- This document uses RFC 2119 terms. ADP-Minimal is a distinct conformance class; ADP-Full implementations MUST also accept Minimal inputs but SHOULD encourage upgrade to Full.

## Required top-level fields
- `adp_version` (must be "0.1.0")
- `id`
- `runtime` (with `execution` containing at least one backend entry)
- `flow` (can be empty `{}` in minimal mode)
- `evaluation` (can be empty `{}` in minimal mode)

## Minimal rules
- `runtime.execution`: at least one entry with `backend` and `id`. Other fields are optional.
- `flow`: empty object `{}` is allowed.
- `evaluation`: empty object `{}` is allowed.

See `examples/minimal/acme-minimal.yaml` for the ACME minimal manifest.
