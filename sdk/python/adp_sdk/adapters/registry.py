"""Adapter registry for framework adapters.

This module provides a central registry for managing and accessing
framework adapters.
"""

from typing import Type

from .base import AdapterBase


class AdapterRegistry:
    """Registry for framework adapters.

    Maintains a mapping of framework IDs to their corresponding adapter classes.
    Provides methods to register, retrieve, and list available adapters.
    """

    _adapters: dict[str, Type[AdapterBase]] = {}

    @classmethod
    def register(cls, adapter_class: Type[AdapterBase]) -> None:
        """Register an adapter class with the registry.

        Args:
            adapter_class: The adapter class to register.
        """
        cls._adapters[adapter_class.framework_id] = adapter_class

    @classmethod
    def get(cls, framework_id: str) -> AdapterBase:
        """Get an instance of a registered adapter.

        Args:
            framework_id: The ID of the framework (e.g., "langgraph", "autogen").

        Returns:
            An instance of the requested adapter.

        Raises:
            ValueError: If the framework ID is not registered.
        """
        if framework_id not in cls._adapters:
            raise ValueError(
                f"Unknown framework: {framework_id}. "
                f"Available frameworks: {', '.join(cls.available())}"
            )
        return cls._adapters[framework_id]()

    @classmethod
    def available(cls) -> list[str]:
        """Get a list of all available framework IDs.

        Returns:
            A list of registered framework IDs.
        """
        return list(cls._adapters.keys())

    @classmethod
    def is_available(cls, framework_id: str) -> bool:
        """Check if a framework adapter is available.

        Args:
            framework_id: The ID of the framework to check.

        Returns:
            True if the adapter is registered, False otherwise.
        """
        return framework_id in cls._adapters
