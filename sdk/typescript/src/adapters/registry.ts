/**
 * Adapter registry for framework adapters.
 *
 * This module provides a central registry for managing and accessing
 * framework adapters.
 */

import { AdapterBase } from "./base";

/**
 * Registry for framework adapters.
 *
 * Maintains a mapping of framework IDs to their corresponding adapter classes.
 * Provides methods to register, retrieve, and list available adapters.
 */
export class AdapterRegistry {
  private static _adapters: Map<string, new () => AdapterBase> = new Map();

  /**
   * Register an adapter class with the registry.
   *
   * @param adapterClass - The adapter class to register
   */
  static register(adapterClass: new () => AdapterBase): void {
    const instance = new adapterClass();
    this._adapters.set(instance.frameworkId, adapterClass);
  }

  /**
   * Get an instance of a registered adapter.
   *
   * @param frameworkId - The ID of the framework (e.g., "langgraph", "autogen")
   * @returns An instance of the requested adapter
   * @throws Error if the framework ID is not registered
   */
  static get(frameworkId: string): AdapterBase {
    const adapterClass = this._adapters.get(frameworkId);
    if (!adapterClass) {
      throw new Error(
        `Unknown framework: ${frameworkId}. ` +
        `Available frameworks: ${Array.from(this._adapters.keys()).join(", ")}`
      );
    }
    return new adapterClass();
  }

  /**
   * Get a list of all available framework IDs.
   *
   * @returns A list of registered framework IDs
   */
  static available(): string[] {
    return Array.from(this._adapters.keys());
  }

  /**
   * Check if a framework adapter is available.
   *
   * @param frameworkId - The ID of the framework to check
   * @returns True if the adapter is registered, False otherwise
   */
  static isAvailable(frameworkId: string): boolean {
    return this._adapters.has(frameworkId);
  }
}
