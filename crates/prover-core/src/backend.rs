//! Provider registration and dispatch.
//!
//! The registry is the seam that keeps proving logic decoupled from concrete
//! backends. Application boundaries (the CLI, adapters) register the backends
//! they want — mock for tests, Noir/UltraHonk for real runs — and everything
//! downstream talks to [`ProviderRegistry`] only.

use crucible_interfaces::{BackendId, ProofProvider, ProofRequest, ProofResponse};

use crate::errors::CoreError;

/// A registry of proof providers, keyed by backend identity.
///
/// Dispatch is **explicit**: [`ProviderRegistry::provide`] fails unless a
/// provider is registered for the request's exact backend, and the provider
/// itself is then asked whether it supports the requested circuit/version.
/// This prevents a request silently falling through to a provider that
/// cannot actually serve it.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ProofProvider>>,
}

impl ProviderRegistry {
    /// Creates an empty registry.
    pub fn new() -> ProviderRegistry {
        ProviderRegistry {
            providers: Vec::new(),
        }
    }

    /// Registers a provider. Later registrations for the same backend replace
    /// earlier ones, so tests can swap in a provider without restarting.
    pub fn register(&mut self, provider: Box<dyn ProofProvider>) {
        self.providers.retain(|p| p.backend() != provider.backend());
        self.providers.push(provider);
    }

    /// Returns the provider registered for `backend`, if any.
    pub fn provider(&self, backend: &BackendId) -> Option<&dyn ProofProvider> {
        self.providers
            .iter()
            .find(|p| p.backend() == *backend)
            .map(|p| p.as_ref())
    }

    /// Returns every registered provider.
    pub fn providers(&self) -> impl Iterator<Item = &dyn ProofProvider> {
        self.providers.iter().map(|p| p.as_ref())
    }

    /// Whether a provider is registered for `backend`.
    pub fn is_registered(&self, backend: &BackendId) -> bool {
        self.provider(backend).is_some()
    }

    /// Dispatches `request` to the provider registered for its backend.
    ///
    /// The provider must also declare support for the requested circuit at
    /// the requested version; both checks fail with distinct errors so a
    /// misconfiguration is obvious.
    pub fn provide(&self, request: &ProofRequest) -> Result<ProofResponse, CoreError> {
        let provider =
            self.provider(&request.backend)
                .ok_or_else(|| CoreError::UnknownBackend {
                    backend: request.backend.clone(),
                })?;
        if !provider.supports(&request.circuit, &request.circuit_version) {
            return Err(CoreError::UnsupportedCircuit {
                backend: request.backend.clone(),
                circuit: request.circuit.clone(),
                version: request.circuit_version,
            });
        }
        provider.generate(request).map_err(CoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_mock::fixtures;

    #[test]
    fn dispatch_routes_to_the_registered_backend() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(crucible_mock::MockProver::new()));
        let request = fixtures::transfer_request();
        let response = registry.provide(&request).unwrap();
        assert_eq!(response.circuit, request.circuit);
        assert_eq!(response.backend.as_str(), BackendId::MOCK);
    }

    #[test]
    fn dispatch_fails_for_unregistered_backends() {
        let registry = ProviderRegistry::new();
        let request = fixtures::transfer_request();
        let err = registry.provide(&request).unwrap_err();
        assert!(matches!(err, CoreError::UnknownBackend { .. }));
    }

    #[test]
    fn later_registration_replaces_earlier() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(crucible_mock::MockProver::with_key("first")));
        registry.register(Box::new(crucible_mock::MockProver::with_key("second")));
        assert_eq!(registry.providers().count(), 1);
        let request = fixtures::transfer_request();
        let response = registry.provide(&request).unwrap();
        // Deterministic mock proof differs by key, so this proves the second
        // provider served the request.
        let direct = crucible_mock::MockProver::with_key("second")
            .generate(&request)
            .unwrap();
        assert_eq!(response, direct);
    }
}
