---
name: develop
description: "Full development pipeline using subagents: requirements → implementation → testing → audit → release"
disable-model-invocation: true
allowed-tools: Task, Bash, Read, Glob, Grep, Write, Edit, TaskCreate, TaskUpdate, TaskList, AskUserQuestion
---

# /develop — Full Development Pipeline

Orchestrate the complete development lifecycle using specialized subagents. This command accepts a feature description and runs it through the entire pipeline.

## Input

The user provides a feature description or development request after `/develop`:

```
/develop <description of what to build or change>
```

The description is available as: **$ARGUMENTS**

## Pipeline Stages

Execute the following stages **sequentially**. After each stage, briefly summarize the outcome to the user before proceeding to the next. If any stage fails, stop and report the failure — do NOT continue to the next stage.

---

### Stage 1: Requirements & Design (requirements-architect)

Launch the **requirements-architect** subagent with this prompt:

> Read the project constitution at `.claude/skills/constitution.md` and existing docs in `docs/`. Then analyze the following request and produce:
> 1. Requirements specification (functional + non-functional)
> 2. Design document with component breakdown
> 3. Work items (epics, user stories with story points, tasks)
>
> Store all artifacts in the `docs/` folder.
>
> Request: $ARGUMENTS

**Wait for completion.** Summarize the requirements and design to the user. Ask the user to confirm before proceeding:

> "Requirements and design are ready. Proceed to implementation?"

If the user says no or wants changes, resume the requirements-architect agent to make revisions.

---

### Stage 2: Implementation (code-implementer)

Launch the **code-implementer** subagent with this prompt:

> Read the project constitution at `skills/constitution.md`. Read all design docs and work items in `docs/` that were just created. Implement the code following:
> - Clean architecture (Domain → Application → Infrastructure)
> - Complete unit test coverage for every public method
> - Beginner-friendly comments on all source code
> - Multi-platform considerations (Windows, Linux, macOS, Web)
> - Scalability and portability best practices
>
> Run `cargo test` (for Rust) and/or `npm test` (for TypeScript) to verify all tests pass before finishing.
>
> Request: $ARGUMENTS

**Wait for completion.** Summarize what was implemented (files created/modified, test results).

---

### Stage 3: Code Review (code-reviewer)

Launch the **code-reviewer** subagent with this prompt:

> Perform an independent code review of the changes just made by the code-implementer agent. You did NOT write this code — review it with fresh eyes.
>
> 1. Read the project constitution at `skills/constitution.md`
> 2. Use `git diff HEAD~1` to identify all changed files (or review uncommitted changes)
> 3. Read the design docs in `docs/` to understand the intent
> 4. Review every changed file for: bugs, security vulnerabilities, architecture compliance, performance issues, test coverage gaps, and constitution compliance
> 5. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` to verify automated checks pass
> 6. Produce a structured Code Review Report with verdict: APPROVED / APPROVED WITH COMMENTS / CHANGES REQUESTED / REJECTED
>
> Feature under review: $ARGUMENTS

**Wait for completion.** Report the review verdict and findings.

If **REJECTED** or **CHANGES REQUESTED** with critical issues: Stop the pipeline. Report issues to the user and ask whether to fix them before proceeding.

If **APPROVED** or **APPROVED WITH COMMENTS**: Proceed to Stage 4.

---

### Stage 4: Integration Testing (integration-test-gatekeeper)

Launch the **integration-test-gatekeeper** subagent with this prompt:

> The code-implementer has finished implementing the following feature. Create an integration test plan, execute all tests, and provide a GO/NO-GO assessment for the changes.
>
> Run the full test suite:
> - `cargo test --manifest-path src/Cargo.toml --workspace` for Rust
> - `npm test` in `src/packages/ui-master` and `src/packages/ui-client` for TypeScript
>
> Also run `cargo clippy --manifest-path src/Cargo.toml --workspace -- -D warnings` and `cargo fmt --manifest-path src/Cargo.toml --all -- --check` to verify lint and formatting.
>
> Feature: $ARGUMENTS

**Wait for completion.** Report the test results and GO/NO-GO decision.

If **NO-GO**: Stop the pipeline. Report failures to the user and suggest fixes.

---

### Stage 5: Quality Audit (agent-outcome-auditor)

Launch the **agent-outcome-auditor** subagent with this prompt:

> Audit the implementation that was just completed for the following feature. Verify:
> 1. All requirements from the design docs are addressed
> 2. Unit tests cover all public methods and edge cases
> 3. Code follows clean architecture principles
> 4. Beginner-friendly comments are present on all new/modified source files
> 5. Multi-platform considerations are handled
> 6. No security vulnerabilities (OWASP top 10)
> 7. Constitution rules (skills/constitution.md) are followed
>
> Report any gaps or issues found.
>
> Feature: $ARGUMENTS

**Wait for completion.** Report audit findings. If critical issues are found, ask the user whether to fix them before proceeding.

---

### Stage 6: Commit & Release Decision

After all stages pass, present a summary to the user:

```
=== Development Pipeline Complete ===

Feature: <description>
Requirements:  Done
Implementation: Done
Tests:          <pass count>/<total count> passed
Audit:          <findings summary>

Ready to commit and push?
```

Ask the user what to do next:
- **Commit & push** — Stage all changes, create a descriptive commit, and push to the current branch
- **Create PR** — Commit, push to a feature branch, and create a pull request
- **Release** — Launch the release-manager agent to tag and release
- **Skip** — Leave changes uncommitted for manual review

---

## Important Rules

1. **Always read `.claude/skills/constitution.md` first** — pass it to every subagent
2. **Sequential execution** — never run stages in parallel; each depends on the previous
3. **Fail fast** — if any stage fails, stop and report; do NOT silently continue
4. **User confirmation** — ask before proceeding to implementation (Stage 2) and before committing (Stage 6)
5. **Track progress** — use TaskCreate/TaskUpdate to show pipeline progress
6. **No hallucination** — only report actual results from subagents, not assumed outcomes
