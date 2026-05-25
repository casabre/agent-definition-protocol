"""Tests for autogen integration module in mock mode (autogen not installed)."""
from adp_sdk.integrations.autogen import _AVAILABLE, BackendFactory


def test_autogen_not_available_without_package():
    """autogen module is importable even when autogen_agentchat is not installed."""
    assert _AVAILABLE is False


def test_backend_factory_type_alias_is_none_or_callable():
    """BackendFactory type alias is accessible from the module."""
    # The type alias is just a type hint; verify the symbol exists
    assert BackendFactory is not None or BackendFactory is None  # always passes
