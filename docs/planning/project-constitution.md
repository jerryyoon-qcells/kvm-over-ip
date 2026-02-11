# Project Constitution
# KVM-Over-IP: Engineering Standards and Principles

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Authoritative - All contributors must follow these guidelines.

---

## 1. Architecture Principles

### 1.1 Clean Architecture (Mandatory)

All code in this project MUST follow Clean Architecture (Robert C. Martin). The dependency rule is absolute:

```
Presentation -> Application -> Domain <- (nothing)
Infrastructure -> Application -> Domain <- (nothing)
```

**Rules**:
- The Domain layer has ZERO dependencies on any other layer or external libraries (except standard library primitives and `uuid`/`serde` for data representation).
- The Application layer depends only on Domain. It contains use cases expressed as plain Rust structs and functions.
- The Infrastructure layer implements interfaces defined in the Application/Domain layer. It is the ONLY layer that touches OS APIs, network sockets, file system, or UI frameworks.
- The Presentation layer (Tauri commands, React components) depends only on Application-layer interfaces, never on infrastructure directly.

**Enforcement**: Code review will reject any PR that violates this dependency rule. If you are unsure which layer something belongs to, ask in the PR.

### 1.2 Dependency Injection (Mandatory)

All infrastructure dependencies in Application-layer use cases MUST be injected as trait objects:

```rust
// Correct: depend on trait, inject concrete type
pub struct RouteInputUseCase {
    transmitter: Arc<dyn InputTransmitter>,
}

// Wrong: depend on concrete type directly
pub struct RouteInputUseCase {
    transmitter: Arc<UdpTransmitter>,  // This violates Clean Architecture
}
```

This makes all use cases fully unit-testable with mock implementations.

### 1.3 Single Responsibility

Each struct/module has ONE reason to change. If you find yourself writing "and also" when describing what a component does, it needs to be split.

---

## 2. Testing Standards (Mandatory)

### 2.1 Coverage Requirements

| Code Category | Minimum Coverage | Measurement |
|---------------|-----------------|-------------|
| Domain entities | 90% | `cargo tarpaulin` |
| Application use cases | 85% | `cargo tarpaulin` |
| Infrastructure adapters | 70% | `cargo tarpaulin` |
| Protocol codec | 95% (every message type) | `cargo tarpaulin` |
| Key translation tables | 100% (all mappings) | `cargo tarpaulin` |
| React components | 80% (via Jest/RTL) | Jest --coverage |

Coverage is measured in CI on every PR. PRs that decrease coverage below these thresholds are rejected.

### 2.2 Test Quality Rules

- **Test names must be descriptive**: `test_cursor_resolves_to_correct_client_when_in_client_region` not `test1`.
- **One logical assertion per test** (multiple `assert!` calls for the same behavior are fine; testing two independent behaviors is not).
- **Arrange-Act-Assert pattern** is required. Use comment blocks to separate sections in non-trivial tests.
- **No sleeps in tests** (`tokio::time::sleep` or `std::thread::sleep` in tests). Use `tokio::time::pause()` and `advance()` for time-dependent logic.
- **No network in unit tests**. Unit tests use mock implementations of network traits.

### 2.3 Test File Conventions

```
// Unit tests: inline in the same file as the code
// tests/unit/ -- for tests that span multiple modules

crates/kvm-core/
  src/
    domain/
      layout.rs           <-- pub mod tests at bottom of file
  tests/
    protocol_roundtrip.rs <-- integration tests for the crate

crates/kvm-master/
  tests/
    routing_integration.rs
    connection_integration.rs
```

---

## 3. Code Style and Quality (Mandatory)

### 3.1 Rust Standards

- **Compiler**: `rustc` stable channel. No nightly features without explicit approval.
- **Clippy**: `cargo clippy -- -D warnings` MUST pass with zero warnings. No `#[allow(clippy::...)]` without an explanatory comment.
- **Formatting**: `cargo fmt` MUST produce no diff. Enforced in CI.
- **Unsafe code**: `unsafe` blocks are forbidden except in the `infrastructure/input_capture` and `infrastructure/input_emulation` modules where OS FFI is necessary. Every `unsafe` block MUST have a `// SAFETY:` comment explaining why it is correct.
- **Error handling**: `unwrap()` and `expect()` are forbidden in non-test code unless preceded by a comment explaining why a panic is impossible in that context. Use `?` and `Result` for fallible operations.
- **Panics**: All public functions that could panic in edge cases must document this with `# Panics` in their rustdoc.

### 3.2 TypeScript/React Standards

- **TypeScript strict mode** (`"strict": true` in tsconfig). No `any` types without explicit comment.
- **ESLint** with `@typescript-eslint/recommended` rules. Zero warnings/errors.
- **Prettier** for formatting. Enforced in CI.
- **Functional components only** in React. No class components.
- **Custom hooks** for any state logic > 5 lines that is used in more than one component.

### 3.3 Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| Rust structs/enums | PascalCase | `KeyEventMessage` |
| Rust functions/methods | snake_case | `encode_message` |
| Rust constants | SCREAMING_SNAKE_CASE | `EDGE_THRESHOLD` |
| Rust traits | PascalCase + verb/noun | `InputTransmitter`, `ScreenEnumerator` |
| TypeScript types/interfaces | PascalCase | `ClientInfo` |
| TypeScript functions | camelCase | `updateLayout` |
| React components | PascalCase | `LayoutEditor` |
| Tauri commands | snake_case | `update_layout` |
| Config keys (TOML) | snake_case | `control_port` |

---

## 4. Git Workflow

### 4.1 Branch Strategy

- `main`: Production-ready code only. Direct pushes forbidden.
- `develop`: Integration branch for in-progress features.
- `feature/US-XXX-short-description`: Feature branches from `develop`.
- `fix/issue-description`: Bug fix branches.
- `release/v1.x.x`: Release preparation branches.

### 4.2 Commit Message Format

All commits must follow Conventional Commits format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`
**Scope**: `kvm-core`, `kvm-master`, `kvm-client`, `kvm-web-bridge`, `ui-master`, `ui-client`, `ci`

**Examples**:
```
feat(kvm-core): implement binary codec for all message types

fix(kvm-client): correct macOS Y-axis coordinate flip for multi-monitor

test(kvm-master): add integration tests for edge transition routing

perf(kvm-master): reduce hook callback latency with lock-free ring buffer
```

### 4.3 Pull Request Requirements

- Every PR requires at least one approving review.
- All CI checks must pass (build, test, lint, format, coverage).
- PR description must reference the story/task: "Implements US-007".
- PRs that add new OS-specific code must be reviewed by a developer who has tested on that OS.

---

## 5. Security Standards (Mandatory)

### 5.1 Sensitive Data Handling

- **Key material** (certificates, session tokens, pairing hashes) MUST NEVER appear in:
  - Log files at INFO or above
  - Error messages returned to UI
  - Stack traces
  - Source code (no hardcoded keys or test credentials committed)
- Use masked representations in logs: `"session_token=<masked>"`.

### 5.2 Input Validation

All data received from the network MUST be validated before use:
- Message length fields must be checked before allocating buffers.
- Sequence numbers must be checked against the replay window.
- Platform codes (VK codes, key codes) must be range-checked before table lookup.
- IPv4 addresses in discovery messages must be validated before connecting.

### 5.3 Dependency Auditing

- `cargo audit` runs in CI on every PR and daily on `main`.
- No new dependencies with known high/critical CVEs.
- `npm audit` runs in CI for frontend dependencies.
- All new Rust dependencies must be reviewed for:
  - Active maintenance (commits in last 12 months)
  - No `unsafe` code without community vetting
  - License compatibility (MIT, Apache-2.0, BSD preferred)

---

## 6. Documentation Standards

### 6.1 Rustdoc Requirements

Every `pub` item must have a rustdoc comment that includes:
- A one-line summary.
- A description of behavior (what it does, not how).
- `# Examples` section for non-trivial functions.
- `# Errors` section for functions returning `Result`.
- `# Panics` section if the function can panic.

### 6.2 Architecture Decision Records (ADRs)

Any significant technical decision (library choice, protocol change, architectural pattern) must be documented as an ADR in `docs/decisions/`.

**ADR Template**:
```
# ADR-XXX: [Title]

## Status: [Proposed | Accepted | Deprecated | Superseded by ADR-YYY]

## Context
[Why does this decision need to be made?]

## Decision
[What was decided?]

## Consequences
[What are the positive and negative consequences?]
```

---

## 7. Performance Standards

### 7.1 Latency Budget

The total latency budget from physical input to emulated event is 10ms (P95). The budget is allocated as:

| Stage | Budget | Measurement |
|-------|--------|-------------|
| Hook callback to queue | 0.5ms | Hook timestamp delta |
| Queue to async runtime | 0.5ms | Channel latency |
| Routing decision | 0.5ms | Use case benchmark |
| Serialization + encryption | 1.0ms | Codec benchmark |
| Network transmission (LAN) | 5.0ms | Network round-trip |
| Deserialization + decryption | 1.0ms | Codec benchmark |
| OS emulation API call | 1.5ms | Platform API benchmark |

Any component that uses more than 150% of its allocated budget without justification must be optimized.

### 7.2 Benchmarking

Critical path code paths MUST have Criterion.rs benchmarks:
- `encode_message` / `decode_message` for all message types
- `VirtualLayout::check_edge_transition`
- `VirtualLayout::resolve_cursor`
- Key code translation functions

Benchmarks run in CI; a > 20% regression blocks the PR.

---

## 8. Cross-Platform Compatibility Matrix

### 8.1 Supported Targets

| Platform | OS Version | Architecture | Support Level |
|----------|-----------|--------------|---------------|
| Windows (Master) | 10 1903+, 11 | x64 | Tier 1 (full support) |
| Windows (Client) | 10 1903+, 11 | x64 | Tier 1 |
| Linux (Client) | Ubuntu 22.04, Debian 12 | x64 | Tier 1 |
| Linux (Client) | Fedora 38+ | x64 | Tier 2 (best effort) |
| macOS (Client) | 12 Monterey+ | arm64 + x64 | Tier 1 |
| Web (Client) | Chrome 100+, Firefox 100+, Edge 100 | N/A | Tier 2 |

**Tier 1**: All features work; all bugs fixed; CI tests run on this target.
**Tier 2**: Core features work; critical bugs fixed; manually tested before release.

### 8.2 Feature Flag for Platform Limitations

Platform-specific limitations must be communicated to users via:
1. Installer pre-requisite checks.
2. In-app error messages with actionable guidance.
3. README limitations section.

Never silently fail on a permissions or compatibility issue. Always surface it to the user.
