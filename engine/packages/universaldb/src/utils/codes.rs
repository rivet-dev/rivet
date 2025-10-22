// === Copied from foundationdbrs ===
pub const NIL: u8 = 0x00;
pub const NESTED: u8 = 0x05;
pub const ESCAPE: u8 = 0xff;

// FDB defines a range (0x40-0x4f) of user type codes for use with its tuple encoding system.
// https://github.com/apple/foundationdb/blob/main/design/tuple.md#user-type-codes

pub const ID: u8 = 0x40;
