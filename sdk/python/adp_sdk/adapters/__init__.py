"""Framework adapters for ADP v0.3.0.

This package provides adapter modules that convert between ADP manifests
and framework-native configuration objects.

Supported frameworks:
- langgraph: LangGraph StateGraph and related constructs
- autogen: AutoGen GroupChat, Assistants, and tools
- crewai: CrewAI agents and tasks
- llamaindex: LlamaIndex QueryEngine and pipelines
- google_adk: Google Agent Development Kit
- openai_agents: OpenAI Agents SDK
- pydantic_ai: Pydantic AI
- semantic_kernel: Semantic Kernel
"""

from .base import AdapterBase
from .registry import AdapterRegistry

# Import all adapters to register them
from . import langgraph  # noqa: F401
from . import autogen  # noqa: F401
from . import crewai  # noqa: F401
from . import llamaindex  # noqa: F401
from . import google_adk  # noqa: F401
from . import openai_agents  # noqa: F401
from . import pydantic_ai  # noqa: F401
from . import semantic_kernel  # noqa: F401

__all__ = ["AdapterBase", "AdapterRegistry"]
