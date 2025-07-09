// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use rustc_version::{Channel, Result, version_meta};

fn main() -> Result<()> {
    let meta = version_meta()?;
    if meta.channel == Channel::Nightly {
        println!("cargo:rustc-cfg=nightly");
    }
    Ok(())
}
