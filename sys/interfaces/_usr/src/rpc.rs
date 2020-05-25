pub type RpcResult<T> = Result<T, RpcError>;

/// A wrapper that hides the ErrorEnum
#[derive(Debug, Copy, Clone)]
pub struct RpcError {
    error: ErrorEnum,
}

impl RpcError {
    pub unsafe fn panic() -> Self {
        Self {
            error: ErrorEnum::PanicUnwind,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum ErrorEnum {
    /// Callee domain is panicked and unwinded
    PanicUnwind,
}
extern crate red_idl;

red_idl::declare_safe_copy!(RpcError);
red_idl::declare_safe_copy!(ErrorEnum);
red_idl::require_copy!(ErrorEnum);
red_idl::require_safe_copy!(ErrorEnum);
