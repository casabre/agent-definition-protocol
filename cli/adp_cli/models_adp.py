"""Pydantic models for ADP manifests."""
from __future__ import annotations

from pathlib import Path

import yaml
from pydantic import BaseModel, Field


class ModelRef(BaseModel):
    provider: str
    name: str
    purpose: str | None = None
    endpoint: str | None = None


class ImageRef(BaseModel):
    registry: str | None = None
    repository: str | None = None
    tag: str | None = None
    digest: str | None = None
    annotations: dict[str, str] = Field(default_factory=dict)


class Runtime(BaseModel):
    framework: str
    entrypoint: str
    language: str
    image: ImageRef | None = None
    models: list[ModelRef]


class Skill(BaseModel):
    id: str
    name: str
    description: str
    tags: list[str] = Field(default_factory=list)
    examples: list[str] = Field(default_factory=list)
    input_modes: list[str]
    output_modes: list[str]
    handler: str | None = None


class A2AInterop(BaseModel):
    inline: dict | None = None
    ref: str | None = None


class Interop(BaseModel):
    a2a: A2AInterop | None = None


class MCPServer(BaseModel):
    id: str
    description: str
    transport: str
    endpoint: str


class HttpAPI(BaseModel):
    id: str
    description: str
    base_url: str
    auth: str | None = None


class SqlFunction(BaseModel):
    id: str
    description: str
    connection: str
    schema: str | None = None


class Tools(BaseModel):
    mcp_servers: list[MCPServer] = Field(default_factory=list)
    http_apis: list[HttpAPI] = Field(default_factory=list)
    sql_functions: list[SqlFunction] = Field(default_factory=list)


class Memory(BaseModel):
    provider: str | None = None
    endpoint: str | None = None
    index: str | None = None
    namespace: str | None = None


class EvalSuite(BaseModel):
    id: str
    runner: str
    config_path: str
    metrics: list[str] = Field(default_factory=list)
    thresholds: dict | None = None


class KPI(BaseModel):
    id: str
    name: str
    description: str
    url: str
    target: str | float | int | None = None
    unit: str | None = None


class PromotionPolicy(BaseModel):
    require_passing_suites: list[str] = Field(default_factory=list)


class Evaluation(BaseModel):
    suites: list[EvalSuite]
    kpis: list[KPI] = Field(default_factory=list)
    promotion_policy: PromotionPolicy


class Governance(BaseModel):
    identity: str | None = None
    allowed_data_scopes: list[str] = Field(default_factory=list)
    forbidden_data_scopes: list[str] = Field(default_factory=list)
    telemetry_endpoint: str | None = None
    guardrail_policy_set: str | None = None


class Environment(BaseModel):
    name: str
    endpoint: str


class Deployment(BaseModel):
    environments: list[Environment]


class ADPManifest(BaseModel):
    adp_version: str
    id: str
    name: str
    description: str
    owner: str
    tags: list[str]
    runtime: Runtime
    skills: list[Skill]
    interop: Interop | None = None
    tools: Tools | None = None
    memory: Memory | None = None
    evaluation: Evaluation
    governance: Governance | None = None
    deployment: Deployment

    model_config = dict(extra="ignore")

    @classmethod
    def from_yaml(cls, path: Path) -> ADPManifest:
        """Load an ADP manifest from a YAML file."""
        data = yaml.safe_load(Path(path).read_text())
        return cls.model_validate(data)

    def to_yaml(self, path: Path | None = None) -> str:
        """Render manifest to YAML; optionally write to file."""
        yaml_str = yaml.safe_dump(self.model_dump(exclude_none=True), sort_keys=False)
        if path:
            Path(path).write_text(yaml_str)
        return yaml_str
