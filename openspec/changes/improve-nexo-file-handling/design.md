# Design: Abstract File Storage for Adapters

## Context
Some adapters (like Nexo) require manual file uploads because APIs are unavailable or insufficient. We need a way to manage these files without burdening the user with path management.

## Architecture

### FileStore Trait
We will introduce a `FileStore` trait in `lib-core` that abstracts the storage and retrieval of user-provided files.

```rust
pub trait FileStore {
    fn save(&self, key: &str, content: &[u8]) -> Result<()>;
    fn load(&self, key: &str) -> Result<Vec<u8>>;
    fn exists(&self, key: &str) -> bool;
}
```

### Managed Storage
Files will be stored in a dedicated, managed directory (e.g., `~/.local/share/crypto-tracker/uploads` or similar, adhering to XDG base directory spec where possible). The user only provides the file once (via CLI or UI), and the system copies/moves it to the managed store.

### Adapter Integration
Adapters like `NexoSvc` will no longer take a file path in their config. Instead, they will request the file content from the `FileStore` using a well-known key (e.g., `nexo/transactions.csv`).

### Environment Variables
For advanced usage (CI/CD, automated setups), we can support loading content from environment variables (e.g., `NEXO_CSV_BASE64`). This can be a fallback or an alternative `FileStore` implementation.

## Security Considerations
- Files should be stored with restricted permissions (0600).
- Future improvements could include encryption at rest.
