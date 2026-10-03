//! Errors, answered with a proper error page.

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use askama::Template;

#[derive(Debug)]
pub enum AppError {
    NotFound,
    /// A template failed to render. Askama checks templates when compiling, so this is rare:
    /// a `Display` implementation returning an error, for instance.
    Render(askama::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("not found"),
            Self::Render(err) => write!(f, "could not render a template: {err}"),
        }
    }
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        Self::Render(err)
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage {
    current_path: &'static str,
    heading: &'static str,
    message: &'static str,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Render(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (heading, message) = match self {
            Self::NotFound => ("Not found", "There is nothing at this address."),
            Self::Render(err) => {
                log::error!("{err}");
                ("Something went wrong", "Please try again in a moment.")
            }
        };
        let page = ErrorPage {
            current_path: "",
            heading,
            message,
        };
        // The error page is a template too; if even that fails, plain text will do.
        match page.render() {
            Ok(html) => HttpResponse::build(self.status_code())
                .content_type("text/html; charset=utf-8")
                .body(html),
            Err(_) => HttpResponse::build(self.status_code()).body(heading),
        }
    }
}
