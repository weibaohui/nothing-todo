//! Tests for database helper functions

#[cfg(test)]
mod unique_constraint_tests {
    fn is_unique_constraint_error(err: &sea_orm::DbErr) -> bool {
        let err_str = format!("{:?}", err);
        err_str.contains("UNIQUE constraint failed")
    }

    #[test]
    fn test_is_unique_constraint_error_with_unique_constraint() {
        let err = sea_orm::DbErr::Query(sea_orm::RuntimeErr::Internal("UNIQUE constraint failed: project_directories.path".to_string()));
        assert!(is_unique_constraint_error(&err));
    }

    #[test]
    fn test_is_unique_constraint_error_without_unique() {
        let err = sea_orm::DbErr::Query(sea_orm::RuntimeErr::Internal("Foreign key constraint failed".to_string()));
        assert!(!is_unique_constraint_error(&err));
    }

    #[test]
    fn test_is_unique_constraint_error_record_not_found() {
        let err = sea_orm::DbErr::RecordNotFound("Record not found".to_string());
        assert!(!is_unique_constraint_error(&err));
    }
}
