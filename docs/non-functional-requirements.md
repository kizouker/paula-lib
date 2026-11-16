# Non-Functional Requirements — `protracker2-agent-assist`

**ID:** `PT2AA-NFR-001` .. `PT2AA-NFR-007`  
**Related Vision:** [docs/vision.md](vision.md) (functional spec: `.kiro/specs/libpt2-core-wrapper/`)

---

## Coverage Table (ISO/IEC 25010:2023)

| Quality Area / Constraint | Status | Notes |
|---|---|---|
| Functional suitability | Open | Functional spec for `libpt2-core-wrapper` exists; functional spec for agent/pattern/UI layer not yet created |
| **Performance efficiency** | **Covered** | Real-time audio rendering, transport latency, non-blocking writes (`PT2AA-NFR-005`) |
| **Compatibility** | **Covered** | Target platform: macOS (`PT2AA-NFR-007`) |
| Interaction capability | **Covered** | Usage documentation / Wiki (`PT2AA-NFR-004`) |
| **Reliability** | **Covered** | Error propagation, crash survival, state integrity (`PT2AA-NFR-006`) |
| **Security** | **Covered** | Basic repository security rules (`PT2AA-NFR-003`) |
| **Maintainability** | **Covered** (targets pending user decision) | Automated CI build (`PT2AA-NFR-001`), branch rules (`PT2AA-NFR-002`) |
| Flexibility (portability) | Covered | CMake build, macOS target platform |
| Safety | Not relevant | — |
| Operations (CI/CD & Docs) | **Covered** | GitHub Actions workflow, branch protection, Wiki |

---

## Priorities
- **Open:** Build time target, branch-rule approval count and audio-device ownership await user confirmation (see USER DECISIONS).
- **Resolved:** Performance, reliability, and compatibility targets are now specified (`PT2AA-NFR-005` through `PT2AA-NFR-007`).

---

## Requirements

### `PT2AA-NFR-001` — Automated CI Build on Check-In
- **Quality Area:** Maintainability / Operations (CI/CD)
- **Source:** User request ("use CI/CD so it builds automatically on check-in using GitHub Actions")
- **Measure (Planguage):**
  - **Scale:** Build pass/fail status per commit push / pull request
  - **Meter:** GitHub Actions workflow run (`.github/workflows/ci.yml`)
  - **Must:** 100% of pushes and pull requests trigger an automated macOS build. An Ubuntu build runs as well, and its failure SHALL NOT block merging.
  - **Goal:** **USER DECISION (proposed default: CI build including tests ≤ 10 minutes).**

```gherkin
Feature: Automated CI build on check-in
  Scenario: Developer pushes code or opens a pull request
    Given the repository contains a valid CMake configuration
    When a commit is pushed or a pull request is opened
    Then GitHub Actions SHALL run the CMake build on macOS (required) and Ubuntu (non-blocking)
    And report a pass or fail check status on the commit
```

---

### `PT2AA-NFR-002` — Branch Protection Rules
- **Quality Area:** Maintainability / Operations
- **Source:** User request ("we should have branch rules")
- **Measure (Planguage):**
  - **Scale:** Protected default branch (`master`) merge policy compliance
  - **Meter:** GitHub repository branch protection / ruleset configuration
  - **Must:** Direct pushes to the default branch are blocked. Changes arrive by pull request with a passing CI check. The required check is the macOS build only.
  - **Goal:** **USER DECISION (proposed default: 0 approvals, passing macOS CI check required).**

```gherkin
Feature: Branch protection on default branch
  Scenario: Merging changes into the default branch
    Given branch protection rules are enabled on the default branch
    When a developer attempts to merge a pull request
    Then the repository SHALL require the CI build check to pass before merging
```

---

### `PT2AA-NFR-003` — Basic Security Rules
- **Quality Area:** Security
- **Source:** User request ("we should also have just basic security rules set up")
- **Measure (Planguage):**
  - **Scale:** Security alerts / secret-scanning coverage on commits and dependencies
  - **Meter:** GitHub Security settings (Dependabot alerts, secret scanning, push protection)
  - **Must:** Secret scanning and dependency vulnerability alerts are enabled on the repository
  - **Goal:** CodeQL C/C++ static analysis may be added later; not required for current scope

```gherkin
Feature: Basic repository security rules
  Scenario: Preventing leaked secrets in commits
    Given the repository settings page is open
    Then secret scanning, push protection and Dependabot alerts SHALL show as enabled
```

---

### `PT2AA-NFR-004` — Usage Wiki / Documentation
- **Quality Area:** Interaction Capability / Maintainability
- **Source:** User request ("we should have a wiki describing how to use it")
- **Measure (Planguage):**
  - **Scale:** Documented core workflows (setup, build, and user usage)
  - **Meter:** GitHub Wiki / `docs/` pages
  - **Must:** At least one getting-started and usage guide exists before release
  - **Goal:** Needs input from user

```gherkin
Feature: Usage documentation in Wiki
  Scenario: New user or contributor reads the Wiki
    Given the project Wiki is published
    When a user opens the Wiki home page
    Then it SHALL describe how to build, configure, and use the application
```

---

### `PT2AA-NFR-005` — Performance Efficiency
- **Quality Area:** Performance Efficiency
- **Source:** Test specialist review — real-time audio with no latency target
- **Measure (Planguage):**
  - **Scale:** Wall-clock time per render call, transport command effect, and render-call duration increase caused by concurrent writes
  - **Meter:** Timer around render calls in automated tests. The test plan defines how the with-writes and without-writes durations are compared.
  - **Must:**
    - Rendering N frames SHALL complete in less than `N / sample_rate` seconds of wall-clock time (real-time constraint)
    - A play or stop command SHALL take effect no later than the render call that starts after the command returns
    - WHILE rendering at 44100 Hz, Pattern Cell writes from another thread SHALL increase the duration of any single render call by no more than 100 µs compared with the same render without writes
  - **Goal:** Render at less than 80% of the real-time budget
  - **Terms:** "Frame" and "render call" are defined in the functional spec glossary.

```gherkin
Feature: Real-time audio performance
  Scenario: Rendering audio within real-time budget
    Given the Audio Engine is initialized with a sample rate of 44100 Hz
    When the Host Client requests 4410 frames (0.1 seconds of audio)
    Then the render call SHALL complete in less than 0.1 seconds of wall-clock time
  Scenario: Transport command effect
    Given the Audio Engine is playing a module
    When the Host Client sends a stop command
    Then the next render call that starts after the command returns SHALL produce silence
  Scenario: Concurrent pattern writes
    Given the Host Client is calling render at 44100 Hz
    When pattern cell writes are made from another thread
    Then the render call duration SHALL increase by no more than 100 microseconds
```

---

### `PT2AA-NFR-006` — Reliability
- **Quality Area:** Reliability
- **Source:** Test specialist review — no crash-recovery or error-propagation requirements
- **Measure (Planguage):**
  - **Scale:** Engine state integrity after error conditions (corrupt files, audio underruns, failed loads)
  - **Meter:** Automated tests that inject corrupt data, delay or skip render calls, and load invalid samples
  - **Must:**
    - A corrupt or truncated module file behaves per Requirement 4.3c (error, previous module unchanged)
    - If a Host Client delays or skips render calls, the Core Wrapper SHALL NOT crash, and the next render call SHALL return the requested number of frames without error
    - A sample load failure behaves per Requirement 4.3b, and all other loaded samples remain unaffected
  - **Decision:** **USER DECISION (proposed default: the library does not own an audio device; the host pulls frames through render).** If the library does own a device, add a device-underrun scenario.
  - **Goal:** All error paths report per Requirement 9 in `.kiro/specs/libpt2-core-wrapper/requirements.md`

```gherkin
Feature: Crash survival and state integrity
  Scenario: Corrupt module file
    Given the Audio Engine has a valid module loaded
    When the Host Client loads a corrupt or truncated module file
    Then the Core Wrapper SHALL return an error status
    And the previously loaded module SHALL remain intact and playable
  Scenario: Delayed render calls
    Given the Audio Engine is playing
    When the Host Client delays or skips render calls
    Then the Core Wrapper SHALL NOT crash
    And the next render call SHALL render correctly
  Scenario: Sample load failure isolation
    Given multiple samples are loaded
    When one sample file fails to load
    Then the Core Wrapper SHALL return an error for that sample
    And all other loaded samples SHALL remain unaffected
```

---

### `PT2AA-NFR-007` — Compatibility
- **Quality Area:** Compatibility
- **Source:** User decision — target platform is macOS for current scope
- **Measure (Planguage):**
  - **Scale:** Build and run success on target platform
  - **Meter:** CI build on `macos-latest` + local verification on macOS
  - **Must:** The project SHALL build and run on macOS (current target platform)
  - **Goal:** Linux CI builds are a bonus, not a requirement; Windows is out of scope for now

```gherkin
Feature: macOS target platform
  Scenario: Building on macOS
    Given the repository is cloned on macOS
    When the developer runs the CMake build
    Then the build SHALL succeed and produce a runnable binary
  Scenario: Linux build as bonus
    Given the CI workflow includes an Ubuntu build
    When the CI runs on push or pull request
    Then the Linux build MAY pass but is not a release requirement
```

---

## Constraints
- Use **CMake** (not Xcode `xcodebuild`, since the repository has no `.xcodeproj` and uses `CMakeLists.txt`).
- Hosted on **GitHub** (GitHub Actions, GitHub Rulesets/Branch Protection, GitHub Security, GitHub Wiki).

## USER DECISIONS
Each item has a proposed default, matching its inline label.

| Req | Decision | Proposed default |
|---|---|---|
| NFR-001 | CI duration target | ≤ 10 minutes |
| NFR-002 | Branch rules | 0 approvals, passing macOS CI check |
| NFR-006 | Library owns an audio device? | No; host pulls frames through render |
