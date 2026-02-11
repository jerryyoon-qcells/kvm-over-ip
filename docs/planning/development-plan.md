# Development Plan
# KVM-Over-IP: Phased Implementation Roadmap

**Document Version**: 1.0
**Date**: 2026-02-10
**Status**: Approved for Execution

---

## 1. Project Overview

**Project Name**: KVM-Over-IP
**Target Version**: 1.0.0
**Development Start**: 2026-02-17 (Week 1)
**Target v1.0 Release**: 2026-09-21 (Week 32)

**Team Composition**:
| Role | Allocation | Responsibilities |
|------|-----------|-----------------|
| Senior Backend Engineer (Rust) | 100% | Core library, master input capture, networking, encryption |
| Cross-Platform Engineer (Rust) | 100% | Client implementations (Windows, Linux, macOS) |
| Frontend Engineer (TypeScript/React) | 100% | Master UI, client UI, web client |
| QA Engineer | 50% | Test harness, integration tests, platform testing |
| DevOps Engineer | 25% | CI/CD pipeline, packaging scripts |

**Sprint Cadence**: 2-week sprints, 10 working days per sprint
**Sprint Velocity (estimated)**: 40 story points per sprint (full team)
**Total Story Points**: 269
**Estimated Sprints**: 7 (with parallel work streams)

---

## 2. Phase Breakdown

### Phase 1: Foundation - Weeks 1-6 (Sprints 1-3)

**Objectives**:
- Establish the Rust workspace structure and CI pipeline.
- Implement `kvm-core` shared library with full test coverage.
- Implement basic master-client TCP connection (no encryption yet).
- Deliver working key transmission between master and Windows client (proof of concept).

**Stories Included**:
| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-042 | CI/CD build pipeline | 8 | DevOps |
| US-001 | Binary message encoding | 5 | Backend |
| US-002 | Message header sequence numbering | 2 | Backend |
| US-003 | Windows VK to HID translation | 5 | Backend |
| US-004 | HID to Linux X11 KeySym translation | 3 | Backend |
| US-005 | HID to macOS CGKeyCode translation | 3 | Backend |
| US-006 | VirtualLayout domain entity | 8 | Backend |
| US-007 | Low-level keyboard hook | 8 | Backend |
| US-008 | Low-level mouse hook | 5 | Backend |
| **Sprint 1-2 Total** | | **47** | |

**Sprint 3 Focus**: Wire up prototype end-to-end

| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-019 | Client UDP broadcast announcement | 3 | Cross-Platform |
| US-020 | Master discovery listener and UI notification | 5 | Backend |
| US-009 | Keyboard event routing | 5 | Backend |
| US-012 | Windows keyboard emulation | 5 | Cross-Platform |
| US-013 | Windows mouse emulation | 5 | Cross-Platform |
| US-010 | Mouse event routing and edge transitions | 8 | Backend |
| **Sprint 3 Total** | | **31** | |

**Deliverables**:
- Working `kvm-core` library with 100% test coverage on all translation tables.
- Unencrypted master -> Windows client keyboard and mouse routing (internal use only).
- Basic discovery broadcast and listening.
- CI pipeline passing on all platforms.

**Milestones**:
- [ ] M1.1: `kvm-core` library compiles and all unit tests pass (Week 2) - 2026-03-02
- [ ] M1.2: Master captures keyboard input and displays in debug log (Week 3) - 2026-03-09
- [ ] M1.3: End-to-end prototype: keypress on master appears in Windows client (Week 6) - 2026-03-30

**Risks & Mitigations**:
- Risk: DTLS library selection takes longer than expected.
  Mitigation: Begin DTLS prototype in parallel with Phase 1; does not block Phase 1 delivery.
- Risk: Windows WH_KEYBOARD_LL hook timing issues.
  Mitigation: Allocate extra buffer week; hook thread priority is documented and tested.

---

### Phase 2: Security & Cross-Platform Clients - Weeks 7-14 (Sprints 4-7)

**Objectives**:
- Implement all security components (TLS, DTLS, certificate management, key storage, pairing).
- Implement Linux and macOS client input emulation.
- Implement screen dimension reporting from all clients.
- Implement layout persistence.

**Stories Included**:

| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-032 | TLS 1.3 control channel | 8 | Backend |
| US-033 | DTLS 1.3 input channel | 8 | Backend |
| US-034 | Secure key storage | 5 | Backend |
| US-021 | PIN-based pairing (master) | 5 | Backend |
| US-022 | PIN-based pairing (client) | 3 | Cross-Platform |
| US-014 | Linux keyboard emulation (XTest) | 5 | Cross-Platform |
| US-015 | Linux mouse emulation (XTest) | 3 | Cross-Platform |
| US-016 | macOS keyboard emulation | 8 | Cross-Platform |
| US-017 | macOS mouse emulation | 5 | Cross-Platform |
| US-018 | Concurrent local + virtual input | 3 | Cross-Platform |
| US-023 | Client screen enumeration and reporting | 5 | Cross-Platform |
| US-024 | Layout persistence and reload | 3 | Backend |
| US-011 | Sharing disable/enable hotkey | 3 | Backend |
| US-043 | Integration test harness | 8 | QA |
| **Phase 2 Total** | | **72** | |

**Deliverables**:
- Fully encrypted master-client communication.
- Working pairing flow with PIN.
- Linux client functional (Ubuntu 22.04 X11).
- macOS client functional (macOS 13).
- Layout configuration persisted to disk.
- Integration test harness operational.

**Milestones**:
- [ ] M2.1: TLS control channel + DTLS input channel operational with tests (Week 9) - 2026-04-27
- [ ] M2.2: Pairing flow end-to-end (master + client) complete (Week 10) - 2026-05-04
- [ ] M2.3: Linux client emulating input from master (Week 12) - 2026-05-18
- [ ] M2.4: macOS client emulating input from master (Week 14) - 2026-06-01

**Risks & Mitigations**:
- Risk: DTLS 1.3 in Rust ecosystem has limited library options.
  Mitigation: Prototype `openssl` FFI vs `webrtc-dtls` in Week 7; decide by Week 8.
- Risk: macOS Accessibility permission complexity causes delays.
  Mitigation: Implement permission check and guidance UI early; allocate 2 extra days.
- Risk: Linux X11 XTest behavior varies across distributions.
  Mitigation: Test on Ubuntu 22.04 (most common) first; expand matrix in Phase 4.

---

### Phase 3: User Interface - Weeks 15-20 (Sprints 8-10)

**Objectives**:
- Build complete master UI (layout editor, client status, settings).
- Build complete client UI (system tray, settings window).
- Implement clipboard sharing.
- Connect all UI components to Tauri commands and events.

**Stories Included**:

| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-026 | Drag-and-drop screen arrangement | 13 | Frontend |
| US-027 | Client status table | 5 | Frontend |
| US-028 | Hotkey configuration UI | 3 | Frontend |
| US-029 | Network settings panel | 3 | Frontend |
| US-030 | System tray icon and status popup | 5 | Frontend |
| US-031 | Client settings window | 5 | Frontend |
| US-025 | Configuration export and import | 3 | Backend |
| US-035 | Master-to-client clipboard push | 5 | Backend + Cross-Platform |
| US-036 | Client-to-master clipboard push | 5 | Backend + Cross-Platform |
| **Phase 3 Total** | | **47** | |

**Deliverables**:
- Fully functional master UI with working drag-and-drop layout editor.
- Fully functional client UI with system tray.
- Clipboard sync working on Windows master + all native clients.

**Milestones**:
- [ ] M3.1: Layout editor drag-and-drop functional with live routing update (Week 17) - 2026-06-22
- [ ] M3.2: Client status table with live latency metrics (Week 18) - 2026-06-29
- [ ] M3.3: All settings panels complete (Week 19) - 2026-07-06
- [ ] M3.4: Clipboard sync working cross-platform (Week 20) - 2026-07-13

**Risks & Mitigations**:
- Risk: Drag-and-drop layout editor is more complex than estimated (13 pts).
  Mitigation: Front-load this story; if > 3 days behind, descope advanced snapping for v1.0.
- Risk: Cross-platform clipboard formats have edge cases.
  Mitigation: Start with plain text only; add HTML and image as stretch goals for v1.0.

---

### Phase 4: Web Client & Polish - Weeks 21-26 (Sprints 11-13)

**Objectives**:
- Implement the web client (WebSocket bridge + browser app).
- Performance optimization based on benchmark results.
- Bug fixing and polish from internal testing.
- Address any remaining cross-platform issues.

**Stories Included**:

| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-037 | Web client WebSocket connection | 8 | Frontend + Backend |
| US-038 | Web client DOM input injection | 5 | Frontend |
| US-044 | Latency performance benchmarks | 5 | QA + Backend |
| **Phase 4 Total** | | **18** | |

**Plus**: Bug fixes and polish sprint (estimated 15 pts of unplanned work buffer).

**Deliverables**:
- Web client functional in Chrome, Firefox, Edge.
- Latency benchmarks passing P95 < 10ms (loopback and LAN).
- All known bugs from Phase 1-3 internal testing resolved.

**Milestones**:
- [ ] M4.1: Web client connects and receives basic input (Week 23) - 2026-07-27
- [ ] M4.2: Latency benchmarks documented and passing (Week 24) - 2026-08-03
- [ ] M4.3: Internal alpha test on 3-machine setup completed (Week 26) - 2026-08-17

---

### Phase 5: Packaging, QA, and Release - Weeks 27-32 (Sprints 14-16)

**Objectives**:
- Create all installer packages.
- Execute full cross-platform test matrix.
- Resolve release-blocking bugs.
- Prepare release artifacts and documentation.

**Stories Included**:

| Story ID | Title | Points | Owner |
|----------|-------|--------|-------|
| US-039 | Windows MSI installers | 5 | DevOps |
| US-040 | Linux DEB/RPM/AppImage | 5 | DevOps |
| US-041 | macOS DMG/PKG installer | 5 | DevOps |
| US-045 | Cross-platform test matrix | 8 | QA |
| **Phase 5 Total** | | **23** | |

**Plus**: Bug fix buffer from test matrix (estimated 20 pts).

**Deliverables**:
- Release-signed installer packages for all platforms.
- Completed test matrix with all test cases passing.
- v1.0.0 tagged release in GitHub.

**Milestones**:
- [ ] M5.1: All installers building successfully in CI (Week 28) - 2026-08-31
- [ ] M5.2: Test matrix 80% complete (Week 30) - 2026-09-07
- [ ] M5.3: All release-blocking bugs resolved (Week 31) - 2026-09-14
- [ ] M5.4: v1.0.0 release published (Week 32) - 2026-09-21

---

## 3. Critical Path

```
[US-042: CI Pipeline]
        |
        v
[US-001: Protocol Codec] --> [US-006: VirtualLayout] --> [US-009: Routing]
                                                               |
[US-007: KB Hook] ----------------------------------------->  |
[US-008: Mouse Hook] ----------------------------------------> |
                                                               v
[US-019: Discovery] --> [US-020: Master Discovery] --> [Pairing Flow]
                                                               |
        [US-032: TLS] ---------------------------------------->|
        [US-033: DTLS] ---------------------------------------->|
                                                               v
        [US-012: Win Emulation] ----> [End-to-End Encrypted Demo]
        [US-014: Linux Emulation] --->              |
        [US-016: macOS Emulation] --->              v
                                            [UI Development]
                                                    |
                                                    v
                                            [Web Client]
                                                    |
                                                    v
                                            [Packaging & QA]
                                                    |
                                                    v
                                             [v1.0 Release]
```

**Critical Path Items** (delays here delay the release date):
1. US-033 (DTLS): High technical risk; must be decided in Week 7.
2. US-006 (VirtualLayout): All routing logic depends on this.
3. US-026 (Layout Editor): Largest single story; front-load in Phase 3.
4. US-016 (macOS emulation): Accessibility permission complexity.

---

## 4. Sprint Plan

### Sprint 1 (Weeks 1-2): Infrastructure Foundation
**Goal**: Project skeleton, CI, and core library started.

| Story | Points | Developer |
|-------|--------|-----------|
| US-042: CI/CD pipeline | 8 | DevOps + Backend |
| US-001: Protocol codec | 5 | Backend |
| US-002: Sequence numbering | 2 | Backend |
| US-003: Windows VK -> HID | 5 | Backend |
| US-004: HID -> X11 KeySym | 3 | Cross-Platform |
| US-005: HID -> CGKeyCode | 3 | Cross-Platform |
| **Sprint Total** | **26** | |

---

### Sprint 2 (Weeks 3-4): Domain & Input Capture
**Goal**: Layout domain entity complete; Windows input hooks working.

| Story | Points | Developer |
|-------|--------|-----------|
| US-006: VirtualLayout entity | 8 | Backend |
| US-007: KB hook (Windows) | 8 | Backend |
| US-008: Mouse hook (Windows) | 5 | Backend |
| **Sprint Total** | **21** | |

---

### Sprint 3 (Weeks 5-6): First End-to-End Prototype
**Goal**: Unencrypted key presses routed from master to Windows client.

| Story | Points | Developer |
|-------|--------|-----------|
| US-019: Client UDP broadcast | 3 | Cross-Platform |
| US-020: Master discovery | 5 | Backend |
| US-009: Keyboard routing | 5 | Backend |
| US-010: Mouse routing + edge transitions | 8 | Backend |
| US-012: Windows KB emulation | 5 | Cross-Platform |
| US-013: Windows mouse emulation | 5 | Cross-Platform |
| **Sprint Total** | **31** | |

---

### Sprint 4 (Weeks 7-8): Security Core
**Goal**: All communication encrypted; pairing flow implemented.

| Story | Points | Developer |
|-------|--------|-----------|
| US-032: TLS 1.3 control channel | 8 | Backend |
| US-033: DTLS 1.3 input channel | 8 | Backend |
| US-034: Secure key storage | 5 | Backend |
| US-021: Pairing flow (master) | 5 | Backend |
| US-022: Pairing flow (client) | 3 | Cross-Platform |
| **Sprint Total** | **29** | |

---

### Sprint 5 (Weeks 9-10): Linux & macOS Clients
**Goal**: All native clients emulating input; screen reporting working.

| Story | Points | Developer |
|-------|--------|-----------|
| US-014: Linux KB emulation | 5 | Cross-Platform |
| US-015: Linux mouse emulation | 3 | Cross-Platform |
| US-016: macOS KB emulation | 8 | Cross-Platform |
| US-017: macOS mouse emulation | 5 | Cross-Platform |
| US-018: Concurrent local input | 3 | Cross-Platform |
| US-023: Screen enumeration & reporting | 5 | Cross-Platform |
| **Sprint Total** | **29** | |

---

### Sprint 6 (Weeks 11-12): Configuration & Testing Foundation
**Goal**: Layout persisted; hotkey working; integration test harness ready.

| Story | Points | Developer |
|-------|--------|-----------|
| US-024: Layout persistence | 3 | Backend |
| US-025: Config export/import | 3 | Backend |
| US-011: Disable/enable hotkey | 3 | Backend |
| US-043: Integration test harness | 8 | QA |
| **Sprint Total** | **17** | (lighter sprint; QA ramp-up) |

---

### Sprint 7 (Weeks 13-14): Master UI - Layout Editor
**Goal**: Working drag-and-drop layout editor connected to backend.

| Story | Points | Developer |
|-------|--------|-----------|
| US-026: Drag-and-drop layout editor | 13 | Frontend |
| US-027: Client status table | 5 | Frontend |
| **Sprint Total** | **18** | |

---

### Sprint 8 (Weeks 15-16): Complete Master UI
**Goal**: All master UI panels complete and functional.

| Story | Points | Developer |
|-------|--------|-----------|
| US-028: Hotkey config UI | 3 | Frontend |
| US-029: Network settings | 3 | Frontend |
| US-035: Master -> client clipboard | 5 | Backend + Frontend |
| **Sprint Total** | **11** | (buffer for layout editor overflow) |

---

### Sprint 9 (Weeks 17-18): Client UI & Clipboard
**Goal**: Client UI complete; clipboard sync working.

| Story | Points | Developer |
|-------|--------|-----------|
| US-030: System tray and status popup | 5 | Frontend |
| US-031: Client settings window | 5 | Frontend |
| US-036: Client -> master clipboard | 5 | Backend + Cross-Platform |
| **Sprint Total** | **15** | |

---

### Sprint 10 (Weeks 19-20): Web Client
**Goal**: Web client connecting and injecting DOM input.

| Story | Points | Developer |
|-------|--------|-----------|
| US-037: Web client WSS connection | 8 | Frontend + Backend |
| US-038: Web client DOM injection | 5 | Frontend |
| US-044: Latency benchmarks | 5 | QA |
| **Sprint Total** | **18** | |

---

### Sprint 11 (Weeks 21-22): Bug Fix & Polish
**Goal**: Address all issues from internal alpha testing.

| Focus | Description |
|-------|-------------|
| Bug fixing | All bugs from Phase 1-4 internal testing |
| Performance | Optimize based on benchmark results |
| Polish | UX improvements from user feedback |

**Buffer**: 20 story points of unplanned work capacity.

---

### Sprint 12 (Weeks 23-24): Packaging
**Goal**: All installer packages building correctly in CI.

| Story | Points | Developer |
|-------|--------|-----------|
| US-039: Windows MSI installers | 5 | DevOps |
| US-040: Linux DEB/RPM/AppImage | 5 | DevOps |
| US-041: macOS DMG/PKG | 5 | DevOps |
| **Sprint Total** | **15** | |

---

### Sprint 13 (Weeks 25-26): QA & Release
**Goal**: Full test matrix executed; v1.0.0 released.

| Story | Points | Developer |
|-------|--------|-----------|
| US-045: Cross-platform test matrix | 8 | QA |
| Release bug fixes | ~12 pts estimated | All |
| **Sprint Total** | **20** | |

---

## 5. Definition of Done

A story is "Done" when ALL of the following are true:

**Code Quality**:
- [ ] Code reviewed by at least one other developer
- [ ] All new code follows the Clean Architecture layer rules (no upward dependencies)
- [ ] No clippy warnings (`cargo clippy -- -D warnings` passes)
- [ ] Code formatted (`cargo fmt` produces no diff)

**Testing**:
- [ ] Unit tests written for all business logic (coverage >= 80% for the module)
- [ ] Integration tests written if the story crosses component boundaries
- [ ] All existing tests still pass (`cargo test` passes on all platforms)
- [ ] Latency-sensitive code has a performance test

**Documentation**:
- [ ] Public API (public functions, structs, traits) has rustdoc comments
- [ ] Non-obvious implementation decisions have inline comments
- [ ] CHANGELOG.md updated with user-visible changes

**Operations**:
- [ ] Changes build successfully in CI for all target platforms
- [ ] No regression in benchmark results (> 20% degradation requires investigation)
- [ ] Feature tested on at least one physical machine (not just unit tests)

---

## 6. Release Criteria for v1.0.0

All of the following must be satisfied:

**Functional**:
- [ ] All Must Have requirements (FR-001 through FR-016, excluding Should Have) are implemented and tested.
- [ ] All test cases in the cross-platform test matrix (US-045) pass.
- [ ] No open P0 (crash) or P1 (data loss, security) bugs.
- [ ] Latency P95 < 10ms verified on standard LAN hardware.

**Quality**:
- [ ] Unit test coverage >= 80% across `kvm-core`, `kvm-master`, and `kvm-client`.
- [ ] All CI pipelines passing (all platform targets).
- [ ] All public APIs documented.

**Packaging**:
- [ ] Windows MSI (master + client) installable on clean Windows 10 and Windows 11 VMs.
- [ ] Linux DEB and AppImage installable on clean Ubuntu 22.04 VM.
- [ ] macOS DMG installable on clean macOS 13 VM (Apple Silicon and Intel).
- [ ] Web client loads in Chrome 120+ and Firefox 120+.

**Documentation**:
- [ ] README with installation and quick-start guide.
- [ ] User guide covering: layout setup, pairing, hotkey configuration.
- [ ] Troubleshooting guide covering common issues (permissions, firewall, discovery).

---

## 7. Risk Register

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-001 | DTLS library selection delays Phase 2 | Medium | High | Prototype in Week 7 (parallel to Sprint 4 start); allocate 2 sprint buffer | Backend Eng |
| R-002 | macOS Accessibility permission UX causes test failures | High | Medium | Implement prominent in-app guidance; test on macOS 12 and 14 early | Cross-Platform Eng |
| R-003 | Linux Wayland users report non-functional client | High | Medium | Document X11 requirement prominently; provide detection and warning | Cross-Platform Eng |
| R-004 | Windows hook timeout under high CPU load | Low | High | Prioritize hook thread; use lock-free queue; load test in Sprint 3 | Backend Eng |
| R-005 | Layout editor 13-pt story overruns Sprint 7 | Medium | Medium | Start early in sprint; descope edge-snapping if behind; add to backlog | Frontend Eng |
| R-006 | macOS code signing/notarization setup time | Medium | Medium | Secure Apple Developer account by Week 22; test signing early | DevOps |
| R-007 | Web client browser security blocks needed APIs | High | Low | Scope web client to DOM events only; do not promise OS-level control | Frontend Eng |
| R-008 | Concurrent input (US-018) has race conditions | Low | High | Test on each platform with simultaneous physical + virtual input | QA |
| R-009 | Network discovery fails on managed corporate networks | Medium | Medium | Document firewall requirements; support manual IP entry as fallback | Backend Eng |

---

## 8. Technology Validation Checkpoints

The following technical decisions must be validated early to avoid late-stage risk:

| Checkpoint | Date | Decision Required | Owner |
|------------|------|-------------------|-------|
| DTLS Library | 2026-03-09 | Choose between `openssl` FFI and `webrtc-dtls` crate | Backend Eng |
| Tauri v2 Compatibility | 2026-02-24 | Verify Tauri v2 supports all required tray features on Linux (GTK) | Frontend Eng |
| XTest vs uinput on Linux | 2026-04-13 | Decide primary Linux emulation method; document privilege requirements | Cross-Platform Eng |
| Web Bridge Architecture | 2026-06-08 | Confirm WebSocket bridge performance is acceptable (< 5ms overhead) | Backend Eng |

---

## 9. Appendix: Feature Flag Strategy

Features that are risky or lower priority can be hidden behind compile-time feature flags to allow the core product to ship without them:

| Feature Flag | Default | Controls |
|-------------|---------|---------|
| `clipboard-sync` | enabled | Clipboard sharing (EPIC-009) |
| `web-client` | enabled | Web client + bridge (EPIC-010) |
| `linux-uinput` | disabled | uinput emulation (alternative to XTest) |
| `debug-overlay` | disabled | On-screen debug latency overlay |

Feature flags allow CI to build and test subsets of the functionality independently.
