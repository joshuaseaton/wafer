// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Compatibility layer for the allocator APIs in the core crate.
//!
//! Provides a uniform interface for allocation APIs that works with both
//! nightly Rust (using `core`) and stable Rust (using `allocator-api2`, the
//! conventional polyfill).

// The `nightly` cfg value is auto-detected and set in the crate's build script.

pub mod alloc {
    #[cfg(nightly)]
    pub use core::alloc::{AllocError, Allocator, Layout};

    #[cfg(nightly)]
    pub use ::alloc::alloc::Global;

    #[cfg(not(nightly))]
    pub use allocator_api2::alloc::{AllocError, Allocator, Global, Layout};

    pub mod collections {
        #[cfg(nightly)]
        pub use ::alloc::collections::TryReserveError;

        #[cfg(not(nightly))]
        pub use allocator_api2::collections::TryReserveError;
    }
}

pub mod boxed {
    #[cfg(nightly)]
    pub use ::alloc::boxed::Box;

    #[cfg(not(nightly))]
    pub use allocator_api2::boxed::Box;
}

pub mod vec {
    #[cfg(nightly)]
    pub use ::alloc::vec::Vec;

    #[cfg(not(nightly))]
    pub use allocator_api2::vec::Vec;
}
