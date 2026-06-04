/**
 * Base adapter class for framework adapters.
 *
 * This module defines the abstract base class that all framework adapters
 * must implement.
 */

import { ADP } from "../adp";

/**
 * Abstract base class for ADP framework adapters.
 *
 * Adapters provide bidirectional conversion between ADP manifests
 * and framework-native configuration objects.
 */
export abstract class AdapterBase {
  /** Unique identifier for the framework (e.g., "langgraph", "autogen") */
  abstract readonly frameworkId: string;

  /**
   * Export ADP manifest to framework-native config object.
   *
   * This method converts an ADP manifest into a framework-specific
   * configuration that can be used directly with the target framework.
   *
   * @param manifest - The ADP manifest to convert
   * @returns A framework-native configuration object
   *
   * @remarks
   * Reads manifest.runtime.adapter_hints[self.frameworkId] for any
   * framework-specific overrides (takes precedence over derived values).
   */
  abstract export(manifest: ADP): Record<string, unknown>;

  /**
   * Import framework-native config into an ADP manifest.
   *
   * This method converts a framework-specific configuration into an ADP
   * manifest. This is a best-effort conversion.
   *
   * @param config - The framework-native configuration object
   * @returns An ADP manifest
   *
   * @remarks
   * Fields with no ADP equivalent MUST be placed in manifest.extensions,
   * never silently discarded (normative requirement).
   */
  abstract importFrom(config: Record<string, unknown>): ADP;

  /**
   * Return coverage per ADP section.
   *
   * Returns a record mapping ADP sections to their fidelity level:
   * - "faithful": Direct mapping, no loss of information
   * - "lossy": Mapping with some information loss
   * - "unsupported": Not supported by this framework
   *
   * @returns Record mapping section names to fidelity levels
   */
  roundtripFidelity(): Record<string, string> {
    // Default implementation - subclasses should override
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
      "sandbox": "unsupported",
      "artifacts": "unsupported",
      "observability": "faithful",
    };
  }
}
