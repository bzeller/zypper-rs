# zypper-rs

Playground implementation of zypper in Rust, goal would be to have a zypper compatible
application but completely rewritten without using any of the C++ zypp codebase.

Contrary to libzypp/zypper this project tries to avoid to reinvent everything, if there is
a crate to do something ( e.g. parser for ini files ), we will use it.

This project is mostly meant as a playground to get to know Rust and maybe get some
inspiration for code refactoring in the original libzypp/zypper codebase.
