RLBoot
======

NOTE: Currently, this is not in a functional state. There is some
hard-to-diagnose bug causing the instruction pointer to go to incorrect locations,
likely related to when we go to real mode to call BIOS interrupts.

RLBoot is meant to be a rewrite of [LBoot](https://github.com/farlepet/lboot)
in Rust. Mainly as a means to get some more hands-on experience with Rust, but
hopefully also add in some addition features that weren't implemented in LBoot.

It's kinda jank. Maybe it'll improve, maybe it won't.

NOTE: Currently, this requires building with a nightly Rust release.
