# Spec: Persistence

## ADDED Requirements

### Requirement: Database Initialization
The system MUST be able to initialize the database schema.

#### Scenario: Initialize Database
Given a new `Store` instance
When `init` is called
Then the `accounts`, `transactions`, and `transaction_effects` tables should exist

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
