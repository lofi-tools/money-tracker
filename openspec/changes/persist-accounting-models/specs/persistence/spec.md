# Spec: Persistence

## ADDED Requirements

### Requirement: Database Initialization
The system MUST be able to initialize the database schema.

#### Scenario: Initialize Database
Given a new `Store` instance
When `init` is called
Then the `accounts`, `transactions`, and `transaction_effects` tables should exist

### Requirement: Ordered idempotent startup migrations
The system SHALL keep schema and data migrations separate from normal storage operations in a dedicated module. Each database open, including in-memory initialization, SHALL execute all migrations in their explicit registry order before returning a usable store. Each migration SHALL be idempotent and SHALL run in its own transaction. A failed step SHALL roll back, stop startup, and report its name; a subsequent startup SHALL safely rerun every step in order.

#### Scenario: Repeated startup
- **WHEN** an already initialized database is opened again
- **THEN** all migration steps execute in order
- **AND** existing records and explicit economic roles remain intact without duplicate backfill rows

#### Scenario: Additional legacy data
- **WHEN** an initialized database gains legacy price observations without coverage
- **AND** the database is reopened
- **THEN** the coverage migration runs again and seeds qualifying dense runs

#### Scenario: Failed step and retry
- **WHEN** a migration fails partway through its transaction
- **THEN** its partial changes are rolled back and startup fails with the step name
- **AND** a later startup can retry safely after the failure is corrected

### Requirement: Account Persistence
The system MUST be able to store and retrieve accounts.

#### Scenario: Save and Retrieve Account
Given a `Store` instance
When I save an `Account`
Then I should be able to retrieve it by ID

### Requirement: Transaction Persistence
The system MUST be able to store and retrieve transactions with their effects.

#### Scenario: Save and Retrieve Transaction
Given a `Store` instance
When I save a `Transaction` with effects
Then I should be able to retrieve it and its effects
