# Non-Functional Requirements — `protracker2-agent-assist`

**ID:** `PT2AA-NFR-001` .. `PT2AA-NFR-004`  
**Related Vision:** [docs/vision.md](vision.md) (no functional spec ID created yet)

---

## Coverage Table (ISO/IEC 25010:2023)

| Quality Area / Constraint | Status | Notes |
|---|---|---|
| Functional suitability | Not relevant | Covered in functional specs |
| Performance efficiency | Open | Real-time audio / Rust DSP targets not yet specified |
| Compatibility | Open | SDL2 vs SDL3, macOS/Linux/Windows targets |
| Interaction capability | **Covered** | Usage documentation / Wiki (`PT2AA-NFR-004`) |
| Reliability | Open | — |
| **Security** | **Covered** | Basic repository security rules (`PT2AA-NFR-003`) |
| **Maintainability** | **Covered** | Automated CI build (`PT2AA-NFR-001`), branch rules (`PT2AA-NFR-002`) |
| Flexibility (portability) | Covered | Cross-platform CMake build on macOS & Linux |
| Safety | Not relevant | — |
| Operations (CI/CD & Docs) | **Covered** | GitHub Actions workflow, branch protection, Wiki |

---

## Priorities
- **Open:** Top 3 quality areas and trade-off ranking not yet specified by user.

---

## Requirements

### `PT2AA-NFR-001` — Automated CI Build on Check-In
- **Quality Area:** Maintainability / Operations (CI/CD)
- **Source:** User request ("use CI/CD so it builds automatically on check-in using GitHub Actions")
- **Measure (Planguage):**
  - **Scale:** Build pass/fail status per commit push / pull request
  - **Meter:** GitHub Actions workflow run (`.github/workflows/ci.yml`)
  - **Must:** 100% of pushes and pull requests trigger an automated CMake build on Linux and macOS
  - **Goal:** Needs input from user (e.g., max build duration target)

```gherkin
Feature: Automated CI build on check-in
  Scenario: Developer pushes code or opens a pull request
    Given the repository contains a valid CMake configuration
    When a commit is pushed or a pull request is opened
    Then GitHub Actions SHALL run the CMake build on Ubuntu and macOS
    And report a pass or fail check status on the commit
```

---

### `PT2AA-NFR-002` — Branch Protection Rules
- **Quality Area:** Maintainability / Operations
- **Source:** User request ("we should have branch rules")
- **Measure (Planguage):**
  - **Scale:** Protected default branch (`master`) merge policy compliance
  - **Meter:** GitHub repository branch protection / ruleset configuration
  - **Must:** Direct pushes to the default branch are blocked or require pull request + passing CI check
  - **Goal:** Needs input from user (e.g., whether solo-dev PR approvals are bypassed or required)

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
  - **Goal:** Needs input from user (e.g., whether CodeQL static analysis is required)

```gherkin
Feature: Basic repository security rules
  Scenario: Preventing leaked secrets in commits
    Given secret scanning and push protection are enabled
    When a commit containing a detected API key or secret is pushed
    Then GitHub SHALL block or alert on the exposed secret
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

## Constraints
- Use **CMake** (not Xcode `xcodebuild`, since the repository has no `.xcodeproj` and uses `CMakeLists.txt`).
- Hosted on **GitHub** (GitHub Actions, GitHub Rulesets/Branch Protection, GitHub Security, GitHub Wiki).

## Open Questions
1. **Build time target:** What is the maximum acceptable CI build duration? *(Needs input from user)*
2. **Branch rules strictness:** For a solo/hackathon repo, should PRs require 0 approvals (just passing CI) or 1 review? *(Needs input from user)*
3. **Security scope:** Should CodeQL C/C++ analysis be enabled in addition to secret scanning and Dependabot? *(Needs input from user)*
4. **Target platforms:** Should Windows (`windows-latest`) also be included in the CI matrix? *(Needs input from user)*
