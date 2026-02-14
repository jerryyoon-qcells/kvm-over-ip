//! Domain entities for KVM-Over-IP.
//!
//! This module contains pure business logic with no infrastructure dependencies.
//!
//! # What is "domain" in Clean Architecture? (for beginners)
//!
//! Clean Architecture organises code into concentric layers.  The innermost
//! layer is called the **domain** (or "entities" layer).  Domain code:
//!
//! - Contains the core business rules of the application.
//! - Has **no** imports from OS APIs, network libraries, database drivers, or UI
//!   frameworks.
//! - Can be compiled and tested on any platform without any external setup.
//! - Defines the data types and operations that make the system uniquely what it
//!   is: in this case, the concept of a virtual screen layout where cursor
//!   movement can cross from one machine to another.
//!
//! Code in outer layers (infrastructure, application, UI) depends on the domain,
//! but the domain never depends on them.  This makes the domain easy to unit-test
//! in isolation.

/// Virtual screen layout — the core domain concept.
///
/// See [`layout::VirtualLayout`] for the main type.
pub mod layout;
