# Contributing

Thank you for considering a contribution! We welcome issues, documentation improvements, and code changes.

## Quick start

1. Fork the repo and create a branch.
2. Make your change with clear commits and tests when applicable.
3. Run `adp validate` on examples if you touch specs or schemas.
4. Open a PR describing the change and rationale.

## Development

- Python 3.12+ recommended.
- CLI code lives under `cli/`; run commands via `python -m adp_cli.main --help` during development.
- Keep specs stable; propose changes via an issue if you need schema updates.

## Code style

- Prefer explicitness over cleverness; include type hints and docstrings.
- Add brief comments only where intent might be unclear.
- Keep dependencies minimal.

## Reporting issues

Open an issue with a clear description, reproduction steps, and expected vs. actual behavior.
