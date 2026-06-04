"""Base adapter class for framework adapters.

This module defines the abstract base class that all framework adapters
must implement.
"""

from abc import ABC, abstractmethod
from typing import Any

from ..adp_model import ADP


class AdapterBase(ABC):
    """Abstract base class for ADP framework adapters.

    Adapters provide bidirectional conversion between ADP manifests
    and framework-native configuration objects.

    Attributes:
        framework_id: Unique identifier for the framework (e.g., "langgraph", "autogen")
    """

    framework_id: str

    @abstractmethod
    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to framework-native config dict.

        This method converts an ADP manifest into a framework-specific
        configuration that can be used directly with the target framework.

        Args:
            manifest: The ADP manifest to convert.

        Returns:
            A dictionary representing the framework-native configuration.

        Notes:
            Reads manifest.runtime.adapter_hints[self.framework_id] for any
            framework-specific overrides (takes precedence over derived values).
        """
        pass

    @abstractmethod
    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import framework-native config into an ADP manifest.

        This method converts a framework-specific configuration into an ADP
        manifest. This is a best-effort conversion.

        Args:
            config: The framework-native configuration dictionary.

        Returns:
            An ADP manifest.

        Notes:
            Fields with no ADP equivalent MUST be placed in manifest.extensions,
            never silently discarded (normative requirement).
        """
        pass

    def roundtrip_fidelity(self) -> dict[str, str]:
        """Return coverage per ADP section.

        Returns a dictionary mapping ADP sections to their fidelity level:
        - "faithful": Direct mapping, no loss of information
        - "lossy": Mapping with some information loss
        - "unsupported": Not supported by this framework

        Returns:
            Dictionary mapping section names to fidelity levels.
        """
        # Default implementation - subclasses should override
        return {
            "flow.graph": "faithful",
            "tools": "faithful",
            "runtime.models": "faithful",
            "tools.policy": "lossy",
            "memory.stores": "lossy",
            "memory.working": "lossy",
            "loop.termination": "lossy",
            "guardrails.interrupts": "lossy",
            "workspace": "unsupported",
            "tools.sandbox": "unsupported",
            "artifacts": "unsupported",
            "observability": "faithful",
        }
