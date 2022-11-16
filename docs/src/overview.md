# Overview

RMM (better name pending) is an **experimental** approach to building cross-platform applications with better testability, higher code and behaviour reuse, better safety and security, and more joy from better tools.

It splits the application into two distinct parts, a Core built in Rust, which drives as much of the business logic as possible, and a Shell, built in the platform native language (Swift, Kotlin, TypeScript), which provides all interfaces with the external world, including the human user, and acts as a platform on which the core runs. 

The interface between the two is a native FFI (Foreign Function Interface) with cross-language type checking and message passing semantics, where simple data structures are passed across the boundary.

## Goals

TO DO.
