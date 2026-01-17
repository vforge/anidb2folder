#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    InvalidArguments = 2,
    DirectoryNotFound = 3,
    MixedFormats = 4,
    UnrecognizedFormat = 5,
    ApiError = 6,
    PermissionError = 7,
    HistoryError = 8,
    RenameError = 9,
    CacheError = 10,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_values() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::GeneralError as i32, 1);
        assert_eq!(ExitCode::InvalidArguments as i32, 2);
        assert_eq!(ExitCode::DirectoryNotFound as i32, 3);
        assert_eq!(ExitCode::MixedFormats as i32, 4);
        assert_eq!(ExitCode::UnrecognizedFormat as i32, 5);
        assert_eq!(ExitCode::ApiError as i32, 6);
        assert_eq!(ExitCode::PermissionError as i32, 7);
        assert_eq!(ExitCode::HistoryError as i32, 8);
        assert_eq!(ExitCode::RenameError as i32, 9);
        assert_eq!(ExitCode::CacheError as i32, 10);
    }

    #[test]
    fn test_exit_code_into_i32() {
        let code: i32 = ExitCode::DirectoryNotFound.into();
        assert_eq!(code, 3);
    }
}
