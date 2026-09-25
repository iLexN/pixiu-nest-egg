# Spec Delta

## Purpose

Describes the backend's machine-readable API contract: a generated OpenAPI document covering the entire `/api` surface and a same-origin interactive playground where the user can inspect and try every endpoint.

## ADDED Requirements

### Requirement: Generated OpenAPI document

The backend SHALL serve an OpenAPI document at `GET /api-docs/openapi.json` that is generated from the compiled route annotations, not from a hand-maintained file. The document SHALL be valid OpenAPI 3.x and SHALL reflect the endpoints as currently compiled — adding or changing a route annotation changes the served document without any separate spec file being edited.

#### Scenario: Fetch the spec

- **WHEN** the user requests `GET /api-docs/openapi.json`
- **THEN** the backend responds `200` with an OpenAPI 3.x JSON document

#### Scenario: Spec tracks the code

- **WHEN** a developer adds, removes, or changes an endpoint annotation and rebuilds the backend
- **THEN** the served document reflects that change with no edits outside the route code

### Requirement: Complete `/api` coverage

The OpenAPI document SHALL describe every endpoint mounted under `/api`: each operation's HTTP method and path, path and query parameters, request body schema where the endpoint accepts one, success response schema, and error responses. Endpoint tags SHALL group operations by domain (stocks, trades, summary, deposits, family deposits, dividends, bonds, MPF, AIA, months, overview/IBKR, year review).

#### Scenario: Every route is documented

- **WHEN** the document is generated
- **THEN** every route registered under `/api` appears as an operation with its method, path, parameters, request body (if any), and success response schema

#### Scenario: Error shape documented

- **WHEN** a client inspects any operation that can fail validation or conflict checks
- **THEN** the operation documents the shared error response shape (status plus field/message payload)

### Requirement: Interactive playground UI

The backend SHALL serve an interactive API playground at `/scalar` on the same origin as the API. The playground SHALL load the OpenAPI document served by the same backend and SHALL let the user send real requests against the local API from the browser.

#### Scenario: Open the playground

- **WHEN** the user navigates to `/scalar` on the running backend
- **THEN** the playground loads, lists the documented endpoints grouped by tag, and shows each operation's parameters and body schema

#### Scenario: Try an endpoint

- **WHEN** the user fills in an operation's inputs in the playground and sends it
- **THEN** the request goes to the same-origin `/api` endpoint and the playground displays the real response status and body

### Requirement: Loopback-only, no new exposure

The spec endpoint and playground SHALL be served only on the backend's existing listener (loopback by default) and SHALL NOT add authentication, new ports, or external network calls at runtime.

#### Scenario: No extra listener

- **WHEN** the backend starts
- **THEN** `/api-docs/openapi.json` and `/scalar` are reachable on the same address as `/api` and no additional port is bound
