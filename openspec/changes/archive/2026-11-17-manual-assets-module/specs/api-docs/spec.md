# Spec Delta

## MODIFIED Requirements

### Requirement: Complete `/api` coverage

The OpenAPI document SHALL describe every endpoint mounted under `/api`: each operation's HTTP method and path, path and query parameters, request body schema where the endpoint accepts one, success response schema, and error responses. Endpoint tags SHALL group operations by domain (stocks, trades, summary, deposits, family deposits, dividends, bonds, MPF, AIA, months, manual assets, overview/IBKR, year review).

#### Scenario: Every route is documented

- **WHEN** the document is generated
- **THEN** every route registered under `/api` appears as an operation with its method, path, parameters, request body (if any), and success response schema

#### Scenario: Error shape documented

- **WHEN** a client inspects any operation that can fail validation or conflict checks
- **THEN** the operation documents the shared error response shape (status plus field/message payload)
