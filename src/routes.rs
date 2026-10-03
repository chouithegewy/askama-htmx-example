//! The pages, and how each answers an ordinary request and an htmx one.
//!
//! Every page is a complete HTML document, so every address works on its own, with
//! JavaScript off, and for search engines. htmx then asks for less where it helps: when the
//! search box asks for results it gets only the results block of the same template, and when
//! the form is sent it gets only the form back, with its errors filled in.
//!
//! htmx marks its requests with headers. The two read here:
//! - `HX-Request: true`, on every request htmx makes;
//! - `HX-Boosted: true`, when it made one only because of `hx-boost` (an ordinary link or form
//!   it took over). Those want the whole page, exactly as without htmx.

use std::sync::Mutex;

use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use askama::Template;
use serde::Deserialize;

use crate::books::{Book, BookForm, FormErrors, Library};
use crate::error::AppError;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(home)
        .service(list_books)
        // Before `/books/{slug}`, or "new" would be taken for a book's slug.
        .service(new_book)
        .service(create_book)
        .service(show_book)
        .service(delete_book)
        .service(about);
}

type Shared = web::Data<Mutex<Library>>;

/// A request htmx made for part of a page, as opposed to a boosted link or no htmx at all.
fn wants_fragment(req: &HttpRequest) -> bool {
    let header = |name| req.headers().get(name).and_then(|v| v.to_str().ok()) == Some("true");
    header("HX-Request") && !header("HX-Boosted")
}

/// The element an htmx request will put the answer into, from `hx-target`.
fn target(req: &HttpRequest) -> Option<&str> {
    req.headers().get("HX-Target").and_then(|v| v.to_str().ok())
}

fn html(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body)
}

fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location))
        .finish()
}

// --- Home -------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    current_path: &'static str,
    count: usize,
    recent: Vec<Book>,
}

#[get("/")]
async fn home(library: Shared) -> Result<HttpResponse, AppError> {
    let books = library.lock().unwrap().search("");
    let page = HomePage {
        current_path: "/",
        count: books.len(),
        recent: books.into_iter().rev().take(3).collect(),
    };
    Ok(html(page.render()?))
}

// --- The list, with search -----------------------------------------------------------------

/// `blocks` makes askama also generate `as_results()`, which renders just the
/// `{% block results %}` of the template: the same markup, never written twice.
#[derive(Template)]
#[template(path = "books.html", blocks = ["results"])]
struct BooksPage {
    current_path: &'static str,
    query: String,
    books: Vec<Book>,
}

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    q: String,
}

#[get("/books")]
async fn list_books(
    req: HttpRequest,
    library: Shared,
    search: web::Query<Search>,
) -> Result<HttpResponse, AppError> {
    let page = BooksPage {
        current_path: "/books",
        books: library.lock().unwrap().search(&search.q),
        query: search.into_inner().q,
    };
    // Checked by target, not just `HX-Request`: going back to a search in the browser's
    // history makes htmx fetch the address again with no target, and that wants the page.
    let body = if wants_fragment(&req) && target(&req) == Some("results") {
        page.as_results().render()?
    } else {
        page.render()?
    };
    let mut response = html(body);
    // The same address answers with a whole page or a part of one, depending on these
    // headers; saying so stops a cache handing the part to someone who wanted the page.
    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static("HX-Request, HX-Target"),
    );
    Ok(response)
}

// --- One book ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "book.html")]
struct BookPage {
    current_path: &'static str,
    book: Book,
}

#[get("/books/{slug}")]
async fn show_book(library: Shared, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    let book = library
        .lock()
        .unwrap()
        .get(&slug)
        .ok_or(AppError::NotFound)?;
    Ok(html(
        BookPage {
            current_path: "/books",
            book,
        }
        .render()?,
    ))
}

/// From the list, htmx sends this and swaps the row for the empty answer, removing it. From
/// the book's own page (a boosted form), or without JavaScript, it goes back to the list.
#[post("/books/{slug}/delete")]
async fn delete_book(
    req: HttpRequest,
    library: Shared,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if !library.lock().unwrap().remove(&slug) {
        return Err(AppError::NotFound);
    }
    Ok(if wants_fragment(&req) {
        html(String::new())
    } else {
        see_other("/books")
    })
}

// --- Adding a book ------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "book_form.html", blocks = ["form"])]
struct BookFormPage {
    current_path: &'static str,
    form: BookForm,
    errors: FormErrors,
}

#[get("/books/new")]
async fn new_book() -> Result<HttpResponse, AppError> {
    let page = BookFormPage {
        current_path: "/books/new",
        form: BookForm::default(),
        errors: FormErrors::default(),
    };
    Ok(html(page.render()?))
}

#[post("/books/new")]
async fn create_book(
    req: HttpRequest,
    library: Shared,
    form: web::Form<BookForm>,
) -> Result<HttpResponse, AppError> {
    let form = form.into_inner();
    match form.check() {
        Ok(new) => {
            let book = library.lock().unwrap().add(new);
            Ok(if wants_fragment(&req) {
                // Tells htmx to fetch the new book's page and show it, as a boosted link
                // would, address bar and history included.
                HttpResponse::Ok()
                    .insert_header(("HX-Location", book.url()))
                    .finish()
            } else {
                see_other(&book.url())
            })
        }
        Err(errors) => {
            let page = BookFormPage {
                current_path: "/books/new",
                form,
                errors,
            };
            Ok(if wants_fragment(&req) {
                // 200, not 422: htmx 2 leaves the page alone on a 4xx unless configured to
                // swap it, and here the swap is the whole point.
                html(page.as_form().render()?)
            } else {
                HttpResponse::UnprocessableEntity()
                    .content_type("text/html; charset=utf-8")
                    .body(page.render()?)
            })
        }
    }
}

// --- About, and the rest ------------------------------------------------------------------

#[derive(Template)]
#[template(path = "about.html")]
struct AboutPage {
    current_path: &'static str,
}

#[get("/about")]
async fn about() -> Result<HttpResponse, AppError> {
    Ok(html(
        AboutPage {
            current_path: "/about",
        }
        .render()?,
    ))
}

pub async fn not_found() -> Result<HttpResponse, AppError> {
    Err(AppError::NotFound)
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;
    use actix_web::{App, test};

    use super::*;

    macro_rules! app {
        () => {
            test::init_service(
                App::new()
                    .app_data(web::Data::new(Mutex::new(Library::with_examples())))
                    .configure(configure)
                    .default_service(web::to(not_found)),
            )
            .await
        };
    }

    async fn body(response: actix_web::dev::ServiceResponse) -> String {
        String::from_utf8(test::read_body(response).await.to_vec()).unwrap()
    }

    #[actix_web::test]
    async fn search_answers_htmx_with_only_the_results() {
        let app = app!();
        let whole = body(
            test::call_service(
                &app,
                test::TestRequest::get().uri("/books?q=moby").to_request(),
            )
            .await,
        )
        .await;
        assert!(
            whole.contains("<html") && whole.contains("Moby-Dick") && !whole.contains("Walden")
        );

        let request = test::TestRequest::get()
            .uri("/books?q=moby")
            .insert_header(("HX-Request", "true"))
            .insert_header(("HX-Target", "results"))
            .to_request();
        let part = body(test::call_service(&app, request).await).await;
        assert!(!part.contains("<html") && part.contains("Moby-Dick"));
    }

    #[actix_web::test]
    async fn a_boosted_request_gets_the_whole_page() {
        let app = app!();
        let request = test::TestRequest::get()
            .uri("/books")
            .insert_header(("HX-Request", "true"))
            .insert_header(("HX-Boosted", "true"))
            .to_request();
        assert!(
            body(test::call_service(&app, request).await)
                .await
                .contains("<html")
        );
    }

    #[actix_web::test]
    async fn the_form_comes_back_with_its_errors_or_goes_to_the_new_book() {
        let app = app!();
        let bad = test::TestRequest::post()
            .uri("/books/new")
            .insert_header(("HX-Request", "true"))
            .set_form([
                ("title", "Emma"),
                ("author", ""),
                ("year", "1815"),
                ("notes", ""),
            ])
            .to_request();
        let response = test::call_service(&app, bad).await;
        assert_eq!(response.status(), StatusCode::OK);
        let form = body(response).await;
        assert!(
            form.starts_with("<form")
                && form.contains("value=\"Emma\"")
                && form.contains("Who wrote it?")
        );

        let without_js = test::TestRequest::post()
            .uri("/books/new")
            .set_form([
                ("title", "Emma"),
                ("author", ""),
                ("year", "1815"),
                ("notes", ""),
            ])
            .to_request();
        assert_eq!(
            test::call_service(&app, without_js).await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );

        let good = test::TestRequest::post()
            .uri("/books/new")
            .insert_header(("HX-Request", "true"))
            .set_form([
                ("title", "Emma"),
                ("author", "Jane Austen"),
                ("year", "1815"),
                ("notes", ""),
            ])
            .to_request();
        let response = test::call_service(&app, good).await;
        assert_eq!(
            response.headers().get("HX-Location").unwrap(),
            "/books/emma"
        );
        let page = test::call_service(
            &app,
            test::TestRequest::get().uri("/books/emma").to_request(),
        )
        .await;
        assert_eq!(page.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn deleting_from_the_list_removes_the_row() {
        let app = app!();
        let request = test::TestRequest::post()
            .uri("/books/walden/delete")
            .insert_header(("HX-Request", "true"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body(response).await.is_empty());

        let again = test::TestRequest::post()
            .uri("/books/walden/delete")
            .to_request();
        assert_eq!(
            test::call_service(&app, again).await.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[actix_web::test]
    async fn text_is_escaped_and_missing_pages_are_404() {
        let app = app!();
        let request = test::TestRequest::get()
            .uri("/books?q=%3Cscript%3E")
            .to_request();
        let page = body(test::call_service(&app, request).await).await;
        assert!(page.contains("&lt;script&gt;") || page.contains("&#60;script&#62;"));
        assert!(!page.contains("<script>alert"));

        let missing = test::TestRequest::get().uri("/nowhere").to_request();
        let response = test::call_service(&app, missing).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(body(response).await.contains("Not found"));
    }
}
