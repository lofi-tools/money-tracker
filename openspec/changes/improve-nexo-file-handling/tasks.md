# Tasks: Improve Nexo File Handling

- [ ] Define `FileStore` trait in `lib-core` for abstract file management. <!-- id: 0 -->
- [ ] Implement `LocalFileStore` in `lib-core` using a managed directory (e.g., `~/.crypto-tracker/data`). <!-- id: 1 -->
- [ ] Update `NexoCsv` to accept a `Reader` or `Content` instead of a path. <!-- id: 2 -->
- [ ] Update `NexoSvc` to use `FileStore` to retrieve the Nexo CSV. <!-- id: 3 -->
- [ ] Implement CLI command `crypto-tracker setup nexo --file <path>` to import the file into the store. <!-- id: 4 -->
- [ ] Add support for `NEXO_CSV_CONTENT` environment variable as a fallback. <!-- id: 5 -->
- [ ] Verify Nexo adapter works with the new storage mechanism. <!-- id: 6 -->
