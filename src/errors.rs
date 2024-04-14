use core::fmt::Display;

#[repr(u8)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ErrorCode {
    Unspecified,
    Unsupported,
    OutOfBounds,
    NoSpace,
    UnsupportedConfig,
    ConfigFormatError,
    UnsupportedExecFmt,
    UnsupportedExecOptions,
    FileNotFound,
    FileUnsupported,
    ReadFailure,

}
impl Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[ErrorCode {}]", *self as u8)
    }
}
