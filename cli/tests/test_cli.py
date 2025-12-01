import zipfile
from pathlib import Path

from typer.testing import CliRunner

from adp_cli.main import app

runner = CliRunner()


MIN_ADP = """
adp_version: "0.2.0"
id: "agent.test"
name: "Test Agent"
description: "A test agent"
owner: "example"
tags: ["test"]
runtime:
  execution:
    - backend: "python"
      id: "python-backend"
      entrypoint: "agent.main:app"
flow: {}
skills:
  - id: "echo"
    name: "Echo"
    description: "Echo input"
    input_modes: ["text"]
    output_modes: ["text"]
evaluation:
  suites:
    - id: "basic"
      description: "Basic eval"
      metrics:
        - id: "deterministic-basic"
          type: "deterministic"
          function: "tests.basic"
          rubric: "echoes back"
          scoring: "boolean"
          threshold: "true"
deployment:
  environments:
    - name: "dev"
      endpoint: "https://dev.example"
"""


MIN_ACS = """
acs_version: "0.1"
base_image: "python:3.12-slim"
build:
  working_dir: "/app"
runtime:
  command: ["python", "-m", "agent"]
"""


def test_validate_success(tmp_path: Path) -> None:
    adp_path = tmp_path / "agent.yaml"
    adp_path.write_text(MIN_ADP)
    acs_path = tmp_path / "container.yaml"
    acs_path.write_text(MIN_ACS)

    result = runner.invoke(app, ["validate", "--adp", str(adp_path), "--acs", str(acs_path)])

    assert result.exit_code == 0, result.stdout
    assert "ADP valid" in result.stdout
    assert "ACS valid" in result.stdout


def test_validate_failure(tmp_path: Path) -> None:
    # Missing required runtime will fail validation
    adp_path = tmp_path / "bad_agent.yaml"
    adp_path.write_text("adp_version: '0.2.0'\nname: missing required fields")

    result = runner.invoke(app, ["validate", "--adp", str(adp_path)])

    assert result.exit_code != 0
    assert "ADP invalid" in result.stdout


def test_pack_and_unpack(tmp_path: Path) -> None:
    adp_path = tmp_path / "agent.yaml"
    adp_path.write_text(MIN_ADP)
    acs_path = tmp_path / "container.yaml"
    acs_path.write_text(MIN_ACS)

    src_dir = tmp_path / "src"
    src_dir.mkdir()
    (src_dir / "main.py").write_text("print('hi')\n")

    eval_dir = tmp_path / "eval"
    eval_dir.mkdir()
    (eval_dir / "basic.yaml").write_text("suite: basic\n")

    out_pkg = tmp_path / "agent.adpkg"

    pack_result = runner.invoke(
        app,
        [
            "pack",
            "--agent",
            str(adp_path),
            "--acs",
            str(acs_path),
            "--src",
            str(src_dir),
            "--eval",
            str(eval_dir),
            "--out",
            str(out_pkg),
        ],
    )
    assert pack_result.exit_code == 0, pack_result.stdout
    assert out_pkg.exists()

    with zipfile.ZipFile(out_pkg) as zf:
        names = set(zf.namelist())
        assert "adp/agent.yaml" in names
        assert "acs/container.yaml" in names
        assert any(n.startswith("src/") for n in names)
        assert any(n.startswith("eval/") for n in names)

    unpack_dir = tmp_path / "unpacked"
    unpack_result = runner.invoke(app, ["unpack", "--pkg", str(out_pkg), "--out", str(unpack_dir)])
    assert unpack_result.exit_code == 0, unpack_result.stdout
    assert (unpack_dir / "adp" / "agent.yaml").exists()
    assert (unpack_dir / "acs" / "container.yaml").exists()
