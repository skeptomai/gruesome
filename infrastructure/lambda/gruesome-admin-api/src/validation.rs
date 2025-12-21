use crate::error::ApiError;

/// Sanitize filename to alphanumeric, dash, and underscore only
pub fn sanitize_filename(filename: &str) -> Result<String, ApiError> {
    if filename.is_empty() {
        return Err(ApiError::BadRequest("Filename cannot be empty".to_string()));
    }

    // Remove path components (security: prevent directory traversal)
    let name = filename
        .split('/')
        .last()
        .unwrap_or(filename)
        .split('\\')
        .last()
        .unwrap_or(filename);

    // Check extension
    if !name.ends_with(".z3")
        && !name.ends_with(".z4")
        && !name.ends_with(".z5")
        && !name.ends_with(".z8")
    {
        return Err(ApiError::BadRequest(
            "File must have .z3, .z4, .z5, or .z8 extension".to_string(),
        ));
    }

    // Sanitize to alphanumeric + dash + underscore + dot
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    if sanitized.is_empty() {
        return Err(ApiError::BadRequest(
            "Filename must contain at least one alphanumeric character".to_string(),
        ));
    }

    Ok(sanitized.to_lowercase())
}

/// Validate game_id (alphanumeric + dash + underscore only)
pub fn validate_game_id(game_id: &str) -> Result<(), ApiError> {
    if game_id.is_empty() {
        return Err(ApiError::BadRequest("Game ID cannot be empty".to_string()));
    }

    if game_id.len() > 50 {
        return Err(ApiError::BadRequest(
            "Game ID must be 50 characters or less".to_string(),
        ));
    }

    if !game_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ApiError::BadRequest(
            "Game ID must contain only alphanumeric characters, dashes, and underscores"
                .to_string(),
        ));
    }

    Ok(())
}

/// Validate title field
pub fn validate_title(title: &str) -> Result<(), ApiError> {
    if title.is_empty() {
        return Err(ApiError::BadRequest("Title cannot be empty".to_string()));
    }

    if title.len() > 100 {
        return Err(ApiError::BadRequest(
            "Title must be 100 characters or less".to_string(),
        ));
    }

    // Check for HTML tags (basic security check)
    if title.contains('<') || title.contains('>') {
        return Err(ApiError::BadRequest(
            "Title cannot contain HTML tags".to_string(),
        ));
    }

    Ok(())
}

/// Validate author field
pub fn validate_author(author: &str) -> Result<(), ApiError> {
    if author.is_empty() {
        return Err(ApiError::BadRequest("Author cannot be empty".to_string()));
    }

    if author.len() > 50 {
        return Err(ApiError::BadRequest(
            "Author must be 50 characters or less".to_string(),
        ));
    }

    // Check for HTML tags
    if author.contains('<') || author.contains('>') {
        return Err(ApiError::BadRequest(
            "Author cannot contain HTML tags".to_string(),
        ));
    }

    Ok(())
}

/// Validate description field
pub fn validate_description(description: &str) -> Result<(), ApiError> {
    if description.is_empty() {
        return Err(ApiError::BadRequest(
            "Description cannot be empty".to_string(),
        ));
    }

    if description.len() > 500 {
        return Err(ApiError::BadRequest(
            "Description must be 500 characters or less".to_string(),
        ));
    }

    // Check for HTML tags
    if description.contains('<') || description.contains('>') {
        return Err(ApiError::BadRequest(
            "Description cannot contain HTML tags".to_string(),
        ));
    }

    Ok(())
}

/// Validate category field (optional)
pub fn validate_category(category: Option<&str>) -> Result<(), ApiError> {
    if let Some(cat) = category {
        let valid_categories = [
            "Adventure",
            "Mystery",
            "Sci-Fi",
            "Fantasy",
            "Humor",
            "Horror",
        ];

        if !valid_categories.contains(&cat) {
            return Err(ApiError::BadRequest(format!(
                "Invalid category: '{}'. Must be one of: {}",
                cat,
                valid_categories.join(", ")
            )));
        }
    }

    Ok(())
}

/// Validate year published (optional, 1977-2025)
pub fn validate_year(year: Option<i32>) -> Result<(), ApiError> {
    if let Some(y) = year {
        if !(1977..=2025).contains(&y) {
            return Err(ApiError::BadRequest(format!(
                "Invalid year: {}. Must be between 1977 and 2025",
                y
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_valid() {
        assert_eq!(sanitize_filename("zork1.z3").unwrap(), "zork1.z3");
        assert_eq!(
            sanitize_filename("ENCHANTER.Z3").unwrap(),
            "enchanter.z3"
        );
        assert_eq!(
            sanitize_filename("zork-2_final.z3").unwrap(),
            "zork-2_final.z3"
        );
    }

    #[test]
    fn test_sanitize_filename_removes_path() {
        assert_eq!(
            sanitize_filename("/path/to/zork1.z3").unwrap(),
            "zork1.z3"
        );
        assert_eq!(
            sanitize_filename("C:\\Games\\zork1.z3").unwrap(),
            "zork1.z3"
        );
    }

    #[test]
    fn test_sanitize_filename_invalid_extension() {
        assert!(sanitize_filename("game.exe").is_err());
        assert!(sanitize_filename("game.txt").is_err());
    }

    #[test]
    fn test_validate_game_id() {
        assert!(validate_game_id("zork1").is_ok());
        assert!(validate_game_id("zork-1").is_ok());
        assert!(validate_game_id("zork_1").is_ok());
        assert!(validate_game_id("").is_err()); // Empty
        assert!(validate_game_id("a".repeat(51).as_str()).is_err()); // Too long
        assert!(validate_game_id("zork@1").is_err()); // Invalid char
    }

    #[test]
    fn test_validate_title() {
        assert!(validate_title("Zork I").is_ok());
        assert!(validate_title("").is_err()); // Empty
        assert!(validate_title(&"a".repeat(101)).is_err()); // Too long
        assert!(validate_title("Zork <script>").is_err()); // HTML tags
    }

    #[test]
    fn test_validate_category() {
        assert!(validate_category(Some("Adventure")).is_ok());
        assert!(validate_category(Some("Sci-Fi")).is_ok());
        assert!(validate_category(None).is_ok()); // Optional
        assert!(validate_category(Some("Invalid")).is_err());
    }

    #[test]
    fn test_validate_year() {
        assert!(validate_year(Some(1980)).is_ok());
        assert!(validate_year(Some(2025)).is_ok());
        assert!(validate_year(None).is_ok()); // Optional
        assert!(validate_year(Some(1976)).is_err()); // Too early
        assert!(validate_year(Some(2026)).is_err()); // Too late
    }
}
