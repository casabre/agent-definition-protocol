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
    execution: list[RuntimeEntry]


class FlowModel(BaseModel):
    """Flow model - can be empty dict or contain flow structure."""

    model_config = dict(extra="allow")


class EvaluationModel(BaseModel):
    """Evaluation model - can be empty dict or contain evaluation structure."""

    model_config = dict(extra="allow")


class ADP(BaseModel):
    adp_version: str
    id: str
    runtime: RuntimeModel
    flow: dict[str, Any] | FlowModel = Field(default_factory=dict)
    evaluation: dict[str, Any] | EvaluationModel = Field(default_factory=dict)
    name: str | None = None
    description: str | None = None

    model_config = dict(extra="allow")

    @classmethod
    def from_file(cls, path: str | Path) -> "ADP":
        data = yaml.safe_load(Path(path).read_text())
        return cls.model_validate(data)

    def to_yaml(self, path: str | Path | None = None) -> str:
        text = yaml.safe_dump(self.model_dump(exclude_none=True), sort_keys=False)
        if path:
            Path(path).write_text(text)
        return text
