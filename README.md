# HermesOS 🧠

## Personal AI Operating System

HermesOS is a personal AI infrastructure project designed to build a long-lived, device-independent digital assistant ecosystem.

The goal is not just to create a chatbot.

The goal is to create a personal operating system where:

- AI agents can reason and plan,
- physical devices can execute tasks,
- knowledge can accumulate over years,
- hardware can be replaced without rebuilding the system.

---

# Vision

HermesOS separates permanent intelligence from temporary hardware.

Devices are considered replaceable worker nodes.

The system keeps:

- identity
- roles
- capabilities
- memory
- configuration

while hardware can change over time.

Example:

```
Old Laptop
    |
    X

New MacBook
    |
    v

Same HermesOS Worker Role
```

---

# Core Architecture

```
                 Hermes Brain

                     |
                     |

              Agent Registry

                     |
                     |

              Worker Agents

                     |
                     |

               Capabilities

                     |
                     |

              Physical Devices
```

Hermes does not directly control devices.

Workers execute tasks according to their permissions.

---

# Main Components

## Hermes Brain

Cloud-based intelligence layer.

Responsibilities:

- task planning
- AI orchestration
- memory retrieval
- decision making


## Worker Agents

Distributed agents running on personal devices.

Examples:

- Raspberry Pi
- Linux machines
- Windows machines
- Future laptops
- Mobile devices


## Capability System

Workers expose controlled abilities.

Examples:

- network monitoring
- file operations
- Docker management
- Home Assistant control
- system information


## Knowledge System

Hermes uses a long-term knowledge architecture.

Structure:

```
knowledge/

├── memory
├── notes
├── people
├── systems
└── decisions
```

The HermesOS repository itself is designed to be used as an Obsidian Vault.

---

# Project Structure

```
HermesOS/

├── docs
│   Documentation and architecture decisions
│
├── workers
│   Agent system
│
├── apps
│   Hermes applications
│
├── knowledge
│   Long-term knowledge base
│
├── config
│   Configuration files
│
├── scripts
│   Automation scripts
│
└── backups
    Backup storage
```

---

# Current Status

## Completed

✅ Project structure created  
✅ Git repository initialized  
✅ Architecture documentation created  
✅ Architecture Decision Records started  
✅ Device Independence principle established  
✅ Obsidian Vault structure created  
✅ Worker architecture planned  


## Next Steps

- Create first Rust Worker Agent
- Design worker identity system
- Build capability framework
- Setup cloud Hermes Brain
- Connect Telegram interface
- Add persistent memory system

---

# Philosophy

## Device Independence

Hardware changes.

Architecture remains.

---

## Human Control

AI should have capabilities, not unlimited permissions.

---

## Long-Term Memory

Knowledge should survive:

- device replacement
- software changes
- model changes

---

## AI Agent Continuation

This project is developed in parallel by multiple AI agents and human developers.

**Rules:**
- ADRs (`docs/adr/`) are the single source of truth for architectural decisions.
- Each AI agent works on its own branch (`agent/<name>`). No direct pushes to master.
- See `docs/08-Branch-Strategy.md` for branch conventions.
- See `rules.md` for all project rules.

**Read first:**

1. docs/07-AI-Handoff.md
2. docs/05-Current-State.md
3. docs/02-Decisions.md
4. docs/08-Branch-Strategy.md

Do not change architectural decisions without creating a new ADR.

---

# Documentation

Current implementation and continuation notes:

- [[docs/05-Current-State|Current State]]
- [[docs/07-AI-Handoff|AI Handoff Guide]]
- [[docs/08-Branch-Strategy|Branch Strategy]]
- [[docs/09-Worker-Protocol|Worker Protocol v1]]
- [[docs/adr/ADR-009-Worker-Enrollment-and-mTLS-Migration|Worker Enrollment and mTLS Migration]]

Start here:

- [[docs/00-Vision|Vision]]
- [[docs/01-Architecture|Architecture]]
- [[docs/02-Decisions|Architecture Decisions]]
- [[docs/07-AI-Handoff|AI Handoff Guide]]

---

# Status

🚧 Active Development

---

# License

To be decided.

---
