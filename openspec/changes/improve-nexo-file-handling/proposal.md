# Proposal: Improve Nexo File Handling UX

## Background
The current Nexo adapter requires a CSV file to be manually downloaded and its path provided. This is a poor user experience. There is no direct API for transaction history, so we must rely on the CSV export.

## Problem
- Users have to manually manage file paths.
- The path is hardcoded or passed as a string, which is brittle.
- No secure storage or abstraction for the file.

## Solution
- Abstract file storage so the user never has to deal with paths after the initial upload.
- Embed file upload in the setup wizard (CLI/UI).
- Store the file in a managed, secure location.
- Support environment variables or secret managers for advanced users.

## Impact
- **UX**: Significantly improved onboarding and maintenance.
- **Security**: Better handling of sensitive data (even if it's just transaction history).
- **Code**: Decoupled adapter from local filesystem paths.
