//! Parser for MovieLens data files.
//!
//! This module will handle parsing the .dat files:
//! - users.dat: userId::gender::age::occupation::zipcode
//! - movies.dat: movieId::title::genres
//! - ratings.dat: userId::movieId::rating::timestamp
//!
//! Rust concepts you'll learn here:
//! - String parsing and splitting
//! - Error handling with `?` operator
//! - Converting between types (parsing strings to numbers)
//! - Working with file I/O
//! - The `FromStr` trait for parsing

use crate::error::{DataLoadError, Result};
use crate::types::*;
use std::io::Read;
use std::path::Path;
use std::fs::File;

/// Helper function to read a file with ISO-8859-1 encoding (Latin-1)
///
/// The MovieLens dataset uses ISO-8859-1 encoding, not UTF-8.
/// This function reads the file as bytes and converts to UTF-8 with lossy conversion.
fn read_lines_latin1(path: &Path) -> Result<Vec<String>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Convert ISO-8859-1 (Latin-1) bytes to UTF-8 string
    // ISO-8859-1 is a single-byte encoding where each byte directly maps to a Unicode code point
    let content: String = bytes.iter().map(|&b| b as char).collect();

    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Parse the users.dat file
///
/// Format: userId::gender::age::occupation::zipcode
///
fn parse_gender(s: &str) -> Result<Gender> {
    match s {
        "M" => Ok(Gender::Male),
        "F" => Ok(Gender::Female),
        _ => Err(DataLoadError::InvalidValue { 
            field: "gender".to_string(), 
            value: s.to_string() 
        }),
    }
}

fn parse_age_group(s: &str) -> Result<AgeGroup> {
    match s {
        "1" => Ok(AgeGroup::Under18),
        "18" => Ok(AgeGroup::Age18To24),
        "25" => Ok(AgeGroup::Age25To34),
        "35" => Ok(AgeGroup::Age35To44),
        "45" => Ok(AgeGroup::Age45To49),
        "50" => Ok(AgeGroup::Age50To55),
        "56" => Ok(AgeGroup::Age56Plus),
        _ => Err(DataLoadError::InvalidValue { 
            field: "age".to_string(), 
            value: s.to_string() 
        }),
    }
}

fn parse_occupation(s: &str) -> Result<Occupation> {
    match s {
        "0" => Ok(Occupation::Other),
        "1" => Ok(Occupation::Academic),
        "2" => Ok(Occupation::Artist),
        "3" => Ok(Occupation::Clerical),
        "4" => Ok(Occupation::CollegeStudent),
        "5" => Ok(Occupation::CustomerService),
        "6" => Ok(Occupation::Doctor),
        "7" => Ok(Occupation::Executive),
        "8" => Ok(Occupation::Farmer),
        "9" => Ok(Occupation::Homemaker),
        "10" => Ok(Occupation::K12Student),
        "11" => Ok(Occupation::Lawyer),
        "12" => Ok(Occupation::Programmer),
        "13" => Ok(Occupation::Retired),
        "14" => Ok(Occupation::Sales),
        "15" => Ok(Occupation::Scientist),
        "16" => Ok(Occupation::SelfEmployed),
        "17" => Ok(Occupation::Technician),
        "18" => Ok(Occupation::Tradesman),
        "19" => Ok(Occupation::Unemployed),
        "20" => Ok(Occupation::Writer),
        _ => Err(DataLoadError::InvalidValue {
            field: "occupation".to_string(),
            value: s.to_string()
        }),
    }
}   

pub fn parse_users(path: &Path) -> Result<Vec<User>> {
    let lines = read_lines_latin1(path)?;
    let mut users = Vec::new();

    // Read line by line
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue; // Skip empty lines
        }
        
        // Split by "::"
        let mut parts = line_trimmed.split("::");
        
        // Parse each field
        let user_id = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "users.dat".to_string(),
                            line: line_no,
                            reason: "Missing userId".to_string(),
                        }
                    )?;
        
        let gender = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "users.dat".to_string(),
                            line: line_no,
                            reason: "Missing gender".to_string(),
                        }
                    )?;

        let age = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "users.dat".to_string(),
                            line: line_no,
                            reason: "Missing age".to_string(),
                        }
                    )?;

        let occupation = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "users.dat".to_string(),
                            line: line_no,
                            reason: "Missing occupation".to_string(),
                        }
                    )?;
        let zipcode = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "users.dat".to_string(),
                            line: line_no,
                            reason: "Missing zipcode".to_string(),
                        }
                    )?;

        // Convert to appropriate types
        let user = User {
            id: user_id.parse().map_err(|e| DataLoadError::ParseError 
                { file: "users.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid userId: {}", e) 
            })?,
            gender: parse_gender(gender)?,
            age: parse_age_group(age)?,
            occupation: parse_occupation(occupation)?,
            zipcode: zipcode.to_string(),
        };

        users.push(user);
    }

    Ok(users)
}

/// Parse the movies.dat file
///
/// Format: movieId::title::genres
///
/// The title often includes year in parentheses: "Toy Story (1995)"
/// Genres are pipe-separated: "Animation|Children's|Comedy"
///
/// TODO: Implement this function
pub fn parse_movies(path: &Path) -> Result<Vec<Movie>> {
    let lines = read_lines_latin1(path)?;
    let mut movies = Vec::new();

    // Read line by line
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue; // Skip empty lines
        }
        
        // Split by "::"
        let mut parts = line_trimmed.split("::");
        
        // Parse each field
        let movie_id = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "movies.dat".to_string(),
                            line: line_no,
                            reason: "Missing movieId".to_string(),
                        }
                    )?;
        
        let title = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "movies.dat".to_string(),
                            line: line_no,
                            reason: "Missing title".to_string(),
                        }
                    )?;

        let genres_str = parts
                    .next()
                    .ok_or_else( || DataLoadError::ParseError { 
                            file: "movies.dat".to_string(),
                            line: line_no,
                            reason: "Missing genres".to_string(),
                        }
                    )?;

        // Convert to appropriate types
        let movie = Movie {
            id: movie_id.parse().map_err(|e| DataLoadError::ParseError 
                { file: "movies.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid movieId: {}", e) 
            })?,
            title: title.to_string(),
            year: extract_year_from_title(title),
            genres: parse_genres(genres_str)?,
        };

        movies.push(movie);
    }
    Ok(movies)
}

/// Parse the ratings.dat file
///
/// Format: userId::movieId::rating::timestamp
///
/// TODO: Implement this function
pub fn parse_ratings(path: &Path) -> Result<Vec<Rating>> {
    let lines = read_lines_latin1(path)?;
    let mut ratings = Vec::new();

    // Read line by line
    for (idx, line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue; // Skip empty lines
        }
        // Split by "::"
        let mut parts = line_trimmed.split("::");

        // Parse each field
        let user_id = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "ratings.dat".to_string(),
                            line: line_no,
                            reason: "Missing userId".to_string(),
                        }
                    )?;
        let movie_id = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "ratings.dat".to_string(),
                            line: line_no,
                            reason: "Missing movieId".to_string(),
                        }
                    )?;
        let rating_value = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "ratings.dat".to_string(),
                            line: line_no,
                            reason: "Missing rating".to_string(),
                        }
                    )?;
        let timestamp = parts
                    .next()
                    .ok_or_else(
                        || DataLoadError::ParseError {
                            file: "ratings.dat".to_string(),
                            line: line_no,
                            reason: "Missing timestamp".to_string(),
                        }
                    )?;
        // Convert to appropriate types
        let rating = Rating {
            user_id: user_id.parse().map_err(|e| DataLoadError::ParseError 
                { file: "ratings.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid userId: {}", e) 
            })?,
            movie_id: movie_id.parse().map_err(|e| DataLoadError::ParseError 
                { file: "ratings.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid movieId: {}", e) 
            })?,
            rating: rating_value.parse().map_err(|e| DataLoadError::ParseError 
                { file: "ratings.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid rating: {}", e) 
            })?,
            timestamp: timestamp.parse().map_err(|e| DataLoadError::ParseError 
                { file: "ratings.dat".to_string(), 
                line: line_no,
                reason: format!("Invalid timestamp: {}", e) 
            })?,
        };

        ratings.push(rating);
    }
    Ok(ratings)
}

/// Extract year from movie title
///
/// Example: "Toy Story (1995)" -> Some(1995)
///          "Movie Title" -> None
fn extract_year_from_title(title: &str) -> Option<u16> {
    // Extract year from parentheses at end of title
    let start = title.rfind('(')?;
    let end = title.rfind(')')?;
    if start < end {
        let year_str = &title[start + 1..end];
        if let Ok(year) = year_str.parse::<u16>() {
            return Some(year);
        }
    }
    None
}

/// Parse a genre string into Genre enum
///
/// Example: "Action" -> Ok(Genre::Action)
///          "Sci-Fi" -> Ok(Genre::SciFi)
fn parse_genre(s: &str) -> Result<Genre> {
    match s {
        "Action" => Ok(Genre::Action),
        "Adventure" => Ok(Genre::Adventure),
        "Animation" => Ok(Genre::Animation),
        "Children's" => Ok(Genre::Children),  // Note: MovieLens uses "Children's" with apostrophe
        "Comedy" => Ok(Genre::Comedy),
        "Crime" => Ok(Genre::Crime),
        "Documentary" => Ok(Genre::Documentary),
        "Drama" => Ok(Genre::Drama),
        "Fantasy" => Ok(Genre::Fantasy),
        "Film-Noir" => Ok(Genre::FilmNoir),
        "Horror" => Ok(Genre::Horror),
        "Musical" => Ok(Genre::Musical),
        "Mystery" => Ok(Genre::Mystery),
        "Romance" => Ok(Genre::Romance),
        "Sci-Fi" => Ok(Genre::SciFi),
        "Thriller" => Ok(Genre::Thriller),
        "War" => Ok(Genre::War),
        "Western" => Ok(Genre::Western),
        _ => Err(DataLoadError::InvalidValue {
            field: "genre".to_string(),
            value: s.to_string()
        }),
    }
}

/// Parse pipe-separated genres
///
/// Example: "Action|Adventure|Sci-Fi" -> vec![Genre::Action, Genre::Adventure, Genre::SciFi]
fn parse_genres(s: &str) -> Result<Vec<Genre>> {
    // Split by "|" and parse each genre
    let mut genres = Vec::new();
    for genre_str in s.split('|') {
        let genre = parse_genre(genre_str)?;
        genres.push(genre);
    }
    Ok(genres)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // #[ignore] // Remove this when you implement extract_year_from_title
    fn test_extract_year() {
        assert_eq!(extract_year_from_title("Toy Story (1995)"), Some(1995));
        assert_eq!(extract_year_from_title("Movie Title"), None);
    }

    #[test]
    // #[should_panic] // Remove this when you implement parse_genre
    fn test_parse_genre() {
        let genre = parse_genre("Action").unwrap();
        assert!(matches!(genre, Genre::Action));
    }
}
