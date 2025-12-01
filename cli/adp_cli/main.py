"""ADP CLI entrypoint."""

from __future__ import annotations

from pathlib import Path
from typing import Annotated

import typer
import yaml
from rich.console import Console

from .packaging_adpkg import pack_adpkg, unpack_adpkg
from .validators import validate_file

app = typer.Typer(help="Agent Definition Protocol CLI")
console = Console()


def _schema_dir() -> Path:
    # Schemas live under /schemas relative to repo root.
    return Path(__file__).resolve().parents[2] / "schemas"


@app.command()
def validate(
    adp: Annotated[Path | None, typer.Option("--adp", help="Path to ADP manifest")] = None,
    acs: Annotated[Path | None, typer.Option("--acs", help="Path to ACS spec")] = None,
) -> None:
    """Validate ADP and/or ACS files against schemas."""
    if not adp and not acs:
        typer.echo("Nothing to validate. Provide --adp and/or --acs.")
        raise typer.Exit(code=1)

    schema_base = _schema_dir()
    success = True
    if adp:
        try:
            validate_file(adp, schema_base / "adp.schema.json")
            console.print(f"✅ ADP valid: {adp}")
        except Exception as exc:  # pragma: no cover - simple surface
            success = False
            console.print(f"❌ ADP invalid: {adp}\n  {exc}")
    if acs:
        try:
            validate_file(acs, schema_base / "acs.schema.json")
            console.print(f"✅ ACS valid: {acs}")
        except Exception as exc:  # pragma: no cover - simple surface
            success = False
            console.print(f"❌ ACS invalid: {acs}\n  {exc}")

    if not success:
        raise typer.Exit(code=1)


@app.command()
def pack(
    agent: Annotated[Path, typer.Option("--agent", help="Path to ADP manifest")],
    acs: Annotated[Path, typer.Option("--acs", help="Path to ACS spec")],
    out: Annotated[Path, typer.Option("--out", help="Output .adpkg path")],
    src: Annotated[Path, typer.Option("--src", help="Source directory")] = Path("./src"),
    eval_dir: Annotated[
        Path, typer.Option("--eval", help="Evaluation directory")
    ] = Path("./eval"),
) -> None:
    """Pack an ADPKG with manifest, container spec, source, and eval assets."""
    if not agent.exists():
        console.print(f"❌ Agent manifest not found: {agent}")
        raise typer.Exit(code=1)
    if not acs.exists():
        console.print(f"❌ ACS spec not found: {acs}")
        raise typer.Exit(code=1)

    pkg_path = pack_adpkg(agent, acs, src, eval_dir, out)
    console.print(f"📦 Created package: {pkg_path}")


@app.command()
def unpack(
    pkg: Annotated[Path, typer.Option("--pkg", help="Path to .adpkg")],
    out: Annotated[Path, typer.Option("--out", help="Output directory")],
) -> None:
    """Unpack an ADPKG into a directory."""
    if not pkg.exists():
        console.print(f"❌ Package not found: {pkg}")
        raise typer.Exit(code=1)
    target = unpack_adpkg(pkg, out)
    console.print(f"📂 Unpacked to: {target}")


@app.command()
def inspect(pkg: Annotated[Path, typer.Option("--pkg", help="Path to .adpkg")]) -> None:
    """Inspect basic info from a package (stub)."""
    if not pkg.exists():
        console.print(f"❌ Package not found: {pkg}")
        raise typer.Exit(code=1)

    try:
        import zipfile

        with zipfile.ZipFile(pkg, "r") as zf:
            agent_yaml = zf.read("adp/agent.yaml").decode()
            agent_data = yaml.safe_load(agent_yaml) or {}
            metadata_raw = (
                zf.read("metadata/version.json")
                if "metadata/version.json" in zf.namelist()
                else None
            )
            metadata = yaml.safe_load(metadata_raw) if metadata_raw else {}
    except Exception as exc:  # pragma: no cover - simple surface
        console.print(f"❌ Failed to inspect package: {exc}")
        raise typer.Exit(code=1) from exc

    console.print("Package contents (stub):")
    console.print(f"- Agent id: {agent_data.get('id', 'unknown')}")
    console.print(f"- Agent name: {agent_data.get('name', 'unknown')}")
    if metadata:
        console.print(f"- Agent version: {metadata.get('agent_version', 'n/a')}")
        console.print(f"- Build timestamp: {metadata.get('build_timestamp', 'n/a')}")
    else:
        console.print("- Metadata: not present")


@app.command(name="build-image")
def build_image_stub() -> None:
    """Placeholder for future container build support."""
    console.print(
        "⚠️  build-image is not implemented yet. Use ACS with your preferred builder."
    )


if __name__ == "__main__":
    app()
