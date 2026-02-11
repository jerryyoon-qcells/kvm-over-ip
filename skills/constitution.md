# Project Constitution

This document defines the foundational rules and principles that govern all development activities in this project.

## 1. Clean Architecture

- Follow clean architecture principles with clear separation of concerns
- Organize code into distinct layers: Presentation, Application, Domain, and Infrastructure
- Dependencies must point inward (outer layers depend on inner layers, never the reverse)
- Domain logic must be independent of frameworks, databases, and external services
- Use dependency injection to maintain loose coupling between components
- Keep business rules isolated and testable without external dependencies

## 2. Unit Testing Requirements

- Maintain complete unit test coverage for all code
- Every public method and function must have corresponding unit tests
- Tests must cover: happy paths, edge cases, error conditions, and boundary values
- Use mocking and stubbing for external dependencies
- Tests must be deterministic, isolated, and repeatable
- Follow the Arrange-Act-Assert (AAA) pattern
- Test names must clearly describe the scenario being tested

## 3. Multi-Platform Deployment

All code must consider deployment across the following platforms:

- **Windows** - Support Windows 10 and later
- **Linux** - Support major distributions (Ubuntu, Debian, Fedora, etc.)
- **macOS** - Support current and previous major versions
- **Web Application** - Support modern browsers (Chrome, Firefox, Safari, Edge)

### Platform Guidelines

- Avoid platform-specific APIs unless properly abstracted
- Use cross-platform libraries and frameworks where possible
- Handle path separators, line endings, and file system differences appropriately
- Test on all target platforms before release
- Document any platform-specific limitations or requirements

### Technology Stack Recommendations

AI must recommend the optimal technology stack for each target environment with clear rationales:

- Provide technology recommendations specific to each platform (Windows, Linux, macOS, Web)
- Each recommendation must include a clear rationale explaining why it is the best choice
- Consider factors such as: performance, maintainability, community support, licensing, and longevity
- Compare alternatives and explain trade-offs when multiple viable options exist
- Recommendations must align with clean architecture principles
- Ensure recommended technologies support cross-platform compatibility where applicable
- Update recommendations when better alternatives emerge with documented justification

## 4. Truthfulness and No Hallucination

- Only state facts that can be verified
- Never fabricate information, data, or capabilities
- Acknowledge uncertainty when information is incomplete
- Cite sources and references when applicable
- If something is unknown, explicitly state "unknown" rather than guessing
- All documentation must accurately reflect the current state of the system

## 5. Failure Reporting

- Report failure explicitly when an operation does not succeed
- Never silently swallow errors or exceptions
- Provide clear, actionable error messages
- Log failures with sufficient context for debugging
- Distinguish between recoverable and non-recoverable failures
- Return appropriate error codes and status indicators
- Failed operations must not leave the system in an inconsistent state

## 6. No Speculation - Only Recommendations

- Do not speculate about outcomes or behaviors
- Base all recommendations on evidence, data, or established best practices
- Clearly label recommendations as such
- Provide rationale for each recommendation
- If data is insufficient for a recommendation, state this explicitly
- Avoid assumptions; when assumptions are necessary, document them clearly
- Recommendations must be actionable and verifiable

## 7. Code Comments for Beginner Developers

- Add detailed comments on all source code so that beginner developers can understand the code
- Explain the purpose and intent of each function, method, class, and module
- Describe the "why" behind non-obvious logic, not just the "what"
- Include comments explaining parameters, return values, and side effects
- Annotate complex algorithms with step-by-step explanations
- Provide context for design pattern usage and architectural decisions
- Comment on any platform-specific or language-specific idioms that may be unfamiliar
- Use clear, plain language — avoid jargon without explanation
