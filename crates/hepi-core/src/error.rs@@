use std::fmt;

/// Mã lỗi §4. Không mã nào mang nghĩa "thử cách khác".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E {
    NoAdmin,
    NoTarget,
    Timeout,
    Access,
    Unsupported,
    State,
}

impl E {
    pub fn code(self) -> &'static str {
        match self {
            E::NoAdmin => "E_NO_ADMIN",
            E::NoTarget => "E_NO_TARGET",
            E::Timeout => "E_TIMEOUT",
            E::Access => "E_ACCESS",
            E::Unsupported => "E_UNSUPPORTED",
            E::State => "E_STATE",
        }
    }
}

impl fmt::Display for E {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for E {}

pub type Res<T> = Result<T, E>;

impl From<windows::core::Error> for E {
    fn from(_: windows::core::Error) -> Self {
        E::Access
    }
}
