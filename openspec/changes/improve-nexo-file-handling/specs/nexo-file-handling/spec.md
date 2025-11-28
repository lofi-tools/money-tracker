# Spec: Nexo File Handling

## ADDED Requirements

### Requirement: Managed File Storage
The system MUST provide a managed storage mechanism for user-uploaded files, abstracting the underlying file system path.

#### Scenario: Import Nexo CSV
Given a user has a Nexo transaction CSV file at `/tmp/nexo.csv`
When the user runs `crypto-tracker setup nexo --import /tmp/nexo.csv`
Then the system copies the file content to the managed store under the key `nexo_transactions`
And the original file is no longer needed for the adapter to function.

### Requirement: Adapter File Retrieval
The Nexo adapter MUST retrieve the transaction CSV content from the managed storage or environment variables, not from a direct user-provided path configuration.

#### Scenario: Fetch Transactions
Given the Nexo CSV is stored in the managed store
When the Nexo adapter `fetch_transactions` is called
Then it retrieves the content from the store
And parses the transactions successfully.

### Requirement: Environment Variable Support
The system MUST allow providing the Nexo CSV content via an environment variable for stateless/automated environments.

#### Scenario: Load from Env Var
Given the `NEXO_CSV_CONTENT` environment variable is set with the CSV content
When the Nexo adapter initializes
Then it uses the content from the environment variable.
