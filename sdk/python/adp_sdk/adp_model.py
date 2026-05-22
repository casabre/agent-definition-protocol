from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml
from pydantic import BaseModel, Field


class RuntimeEntry(BaseModel):
    backend: str
    id: str
    entrypoint: str | list[str] | None = None
    image: str | None = None
    module: str | None = None
    env: dict[str, str] = Field(default_factory=dict)


class RuntimeModel(BaseModel):
    model_config = dict(extra="allow")
    execution: list[RuntimeEntry]


class FlowModel(BaseModel):
    model_config = dict(extra="allow")


class EvaluationModel(BaseModel):
    model_config = dict(extra="allow")


class GuardrailRail(BaseModel):
    id: str
    provider: str
    policy_ref: str
    mode: str | None = None
    categories: list[str] | None = None
    threshold: str | None = None
    model_config = dict(extra="allow")


class Guardrails(BaseModel):
    input: list[GuardrailRail] = Field(default_factory=list)
    output: list[GuardrailRail] = Field(default_factory=list)
    on_violation: str | None = None


class Telemetry(BaseModel):
    endpoint: str | None = None
    protocol: str | None = None
    service_name: str | None = None
    sampling_rate: float | None = None
    required_attributes: list[str] = Field(default_factory=list)


class ImportEntry(BaseModel):
    model_config = dict(populate_by_name=True)
    id: str
    from_uri: str = Field(..., alias="from")
    sections: list[str] = Field(default_factory=list)


class OverrideEntry(BaseModel):
    path: str
    value: Any = None
    op: str = "set"


class ADP(BaseModel):
    adp_version: str
    id: str
    runtime: RuntimeModel
    flow: dict[str, Any] | FlowModel = Field(default_factory=dict)
    evaluation: dict[str, Any] | EvaluationModel = Field(default_factory=dict)
    name: str | None = None
    description: str | None = None
    extends: str | None = None
    imports: list[ImportEntry] | None = Field(None, alias="import")
    overrides: list[OverrideEntry] | None = None
    guardrails: Guardrails | None = None
    telemetry: Telemetry | None = None

    model_config = dict(extra="allow", populate_by_name=True)

    @classmethod
    def from_file(cls, path: str | Path) -> "ADP":
        data = yaml.safe_load(Path(path).read_text())
        return cls.model_validate(data)

    def to_yaml(self, path: str | Path | None = None) -> str:
        text = yaml.safe_dump(
            self.model_dump(by_alias=True, exclude_none=True), sort_keys=False
        )
        if path:
            Path(path).write_text(text)
        return text
