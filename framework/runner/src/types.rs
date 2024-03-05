/// Recommended error type for your scenario `main` function and any shared behaviour code that you
/// write for hooks. This type is compatible with the [crate::definition::HookResult] type so you can
/// use `?` to propagate errors.
pub type WindTunnelResult<T> = anyhow::Result<T>;
