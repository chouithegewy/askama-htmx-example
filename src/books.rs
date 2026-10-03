//! The books, and checking what someone typed into the form before it becomes one.

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Book {
    pub slug: String,
    pub title: String,
    pub author: String,
    pub year: i32,
    pub notes: String,
}

impl Book {
    pub fn url(&self) -> String {
        format!("/books/{}", self.slug)
    }
}

/// Every book, in the order they were added.
#[derive(Debug, Default)]
pub struct Library {
    books: Vec<Book>,
}

impl Library {
    /// A few public-domain classics to start from.
    pub fn with_examples() -> Self {
        let mut library = Self::default();
        for (title, author, year, notes) in [
            (
                "Pride and Prejudice",
                "Jane Austen",
                1813,
                "Manners, money and first impressions.",
            ),
            (
                "Frankenstein",
                "Mary Shelley",
                1818,
                "A creation, and the creator who runs from it.",
            ),
            (
                "Moby-Dick",
                "Herman Melville",
                1851,
                "One captain, one whale, and a great deal about rope.",
            ),
            (
                "Walden",
                "Henry David Thoreau",
                1854,
                "Two years in a cabin by a pond.",
            ),
            (
                "Middlemarch",
                "George Eliot",
                1871,
                "A provincial town, seen whole.",
            ),
        ] {
            library.add(NewBook {
                title: title.to_owned(),
                author: author.to_owned(),
                year,
                notes: notes.to_owned(),
            });
        }
        library
    }

    /// The books whose title or author contains `query`, ignoring case; all of them if it is
    /// empty.
    pub fn search(&self, query: &str) -> Vec<Book> {
        let query = query.trim().to_lowercase();
        self.books
            .iter()
            .filter(|book| {
                query.is_empty()
                    || book.title.to_lowercase().contains(&query)
                    || book.author.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    pub fn get(&self, slug: &str) -> Option<Book> {
        self.books.iter().find(|book| book.slug == slug).cloned()
    }

    /// Adds a book, giving it an address of its own ("moby-dick", "moby-dick-2", …).
    pub fn add(&mut self, new: NewBook) -> Book {
        let base = slugify(&new.title);
        let mut slug = base.clone();
        let mut n = 2;
        while self.books.iter().any(|book| book.slug == slug) {
            slug = format!("{base}-{n}");
            n += 1;
        }
        let book = Book {
            slug,
            title: new.title,
            author: new.author,
            year: new.year,
            notes: new.notes,
        };
        self.books.push(book.clone());
        book
    }

    /// Whether there was such a book to remove.
    pub fn remove(&mut self, slug: &str) -> bool {
        let before = self.books.len();
        self.books.retain(|book| book.slug != slug);
        self.books.len() != before
    }
}

/// The form as it was posted: every field a string, exactly as typed, so it can be shown
/// back unchanged when something in it needs fixing.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct BookForm {
    pub title: String,
    pub author: String,
    pub year: String,
    pub notes: String,
}

/// A form that has been checked.
#[derive(Debug)]
pub struct NewBook {
    pub title: String,
    pub author: String,
    pub year: i32,
    pub notes: String,
}

/// What is wrong with each field, if anything.
#[derive(Debug, Default)]
pub struct FormErrors {
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<String>,
}

impl BookForm {
    pub fn check(&self) -> Result<NewBook, FormErrors> {
        let mut errors = FormErrors::default();
        let title = self.title.trim();
        let author = self.author.trim();
        if title.is_empty() {
            errors.title = Some("Every book needs a title.".to_owned());
        }
        if author.is_empty() {
            errors.author = Some("Who wrote it? \"Anonymous\" is fine.".to_owned());
        }
        let year = match self.year.trim().parse::<i32>() {
            Ok(year) if (-3000..=2100).contains(&year) => Some(year),
            Ok(_) => {
                errors.year = Some("That year looks wrong.".to_owned());
                None
            }
            Err(_) => {
                errors.year = Some("The year, as a number, e.g. 1851.".to_owned());
                None
            }
        };
        match (errors.title.is_none() && errors.author.is_none(), year) {
            (true, Some(year)) => Ok(NewBook {
                title: title.to_owned(),
                author: author.to_owned(),
                year,
                notes: self.notes.trim().to_owned(),
            }),
            _ => Err(errors),
        }
    }
}

fn slugify(text: &str) -> String {
    let slug = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "book".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ignores_case_and_matches_authors() {
        let library = Library::with_examples();
        assert_eq!(library.search("MOBY").len(), 1);
        assert_eq!(library.search("shelley")[0].title, "Frankenstein");
        assert_eq!(library.search("").len(), 5);
    }

    #[test]
    fn titles_that_repeat_get_their_own_address() {
        let mut library = Library::default();
        let form = BookForm {
            title: "Walden".into(),
            author: "H. D. Thoreau".into(),
            year: "1854".into(),
            notes: String::new(),
        };
        assert_eq!(library.add(form.check().unwrap()).slug, "walden");
        assert_eq!(library.add(form.check().unwrap()).slug, "walden-2");
    }

    #[test]
    fn a_bad_form_says_what_is_wrong_with_each_field() {
        let errors = BookForm {
            year: "soon".into(),
            ..Default::default()
        }
        .check()
        .unwrap_err();
        assert!(errors.title.is_some() && errors.author.is_some() && errors.year.is_some());
    }
}
