"""Pydantic models for ACS specs."""
from __future__ import annotations

from pathlib import Path

import yaml
from pydantic import BaseModel, Field


class CopySpec(BaseModel):
    src: str
    dest: str


class Dependencies(BaseModel):
    python: list[str] = Field(default_factory=list)
    system: list[str] = Field(default_factory=list)


class Build(BaseModel):
    working_dir: str
    copy: list[CopySpec] = Field(default_factory=list)
    dependencies: Dependencies = Field(default_factory=Dependencies)


class Healthcheck(BaseModel):
    path: str | None = None
    interval_seconds: int | None = None
    timeout_seconds: int | None = None


class Runtime(BaseModel):
    command: list[str]
    args: list[str] = Field(default_factory=list)
    env: list[str] = Field(default_factory=list)
    ports: list[int] = Field(default_factory=list)
    healthcheck: Healthcheck | None = None


class Otel(BaseModel):
    enabled: bool | None = None
    endpoint_env: str | None = None
    service_name: str | None = None


class TelemetryBindings(BaseModel):
    otel: Otel | None = None


class EvalOnStartup(BaseModel):
    run_suites: list[str] = Field(default_factory=list)


class EvalOnDeploy(BaseModel):
    require_passing_suites: list[str] = Field(default_factory=list)


class EvalBindings(BaseModel):
    on_startup: EvalOnStartup = Field(default_factory=EvalOnStartup)
    on_deploy: EvalOnDeploy = Field(default_factory=EvalOnDeploy)


class ACSSpec(BaseModel):
    acs_version: str
    base_image: str
    build: Build
    runtime: Runtime
    telemetry_bindings: TelemetryBindings = Field(default_factory=TelemetryBindings)
    eval_bindings: EvalBindings = Field(default_factory=EvalBindings)

    model_config = dict(extra="ignore")

    @classmethod
    def from_yaml(cls, path: Path) -> ACSSpec:
        """Load an ACS spec from a YAML file."""
        data = yaml.safe_load(Path(path).read_text())
        return cls.model_validate(data)

    def to_yaml(self, path: Path | None = None) -> str:
        """Render ACS spec to YAML; optionally write to file."""
        yaml_str = yaml.safe_dump(self.model_dump(exclude_none=True), sort_keys=False)
        if path:
            Path(path).write_text(yaml_str)
        return yaml_str
