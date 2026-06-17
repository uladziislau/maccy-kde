use tokio::runtime::Runtime;
use crate::shared::Result;

pub struct DaemonRuntime {
    runtime: Option<Runtime>,
}

impl DaemonRuntime {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| crate::shared::MaccyError::Database(format!("Failed to create runtime: {}", e)))?;
        
        Ok(Self {
            runtime: Some(runtime),
        })
    }

    pub fn block_on<F, R>(&self, f: F) -> R
    where
        F: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        self.runtime.as_ref().unwrap().block_on(f)
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.runtime.as_ref().unwrap().spawn(f);
    }

    pub fn shutdown(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

impl Drop for DaemonRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Default for DaemonRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = DaemonRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_runtime_default() {
        let runtime = DaemonRuntime::default();
        let result = runtime.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_runtime_spawn() {
        use std::sync::Arc;
        let runtime = DaemonRuntime::default();
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        runtime.spawn(async move {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        
        // Give the spawned task time to complete
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn test_runtime_shutdown() {
        let mut runtime = DaemonRuntime::default();
        runtime.shutdown();
        // Runtime should be usable after shutdown for cleanup
    }
}