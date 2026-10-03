# askama + htmx example

A small reading list, as an example of setting up a server-rendered Rust site:

- **[actix-web](https://actix.rs)** answers the requests;
- **[askama](https://askama.rs)** compiles the HTML templates into Rust, so a mistake in a
  template is a compile error rather than a broken page;
- **[htmx](https://htmx.org)** fetches pages and parts of pages in the background, so the site
  feels like an app with no JavaScript framework and no build step.

Every page is a complete HTML document and works with JavaScript turned off; htmx only makes
it quicker.

## Run it

```sh
cargo run
```

Then open <http://127.0.0.1:8080>. Set `ADDRESS` to use another address or port, e.g.
`ADDRESS=127.0.0.1:3000 cargo run`; `RUST_LOG=debug` shows more of what the server is doing.
The books live in memory, so a restart puts the examples back.

```sh
cargo test
```

tests each page with and without the headers htmx sends.

## What is where

```
src/
  main.rs        server setup: shared state, routes, static files, the 404 page
  routes.rs      the pages, and how each answers an ordinary request and an htmx one
  books.rs       the data, and checking the form
  nav.rs         the site's name and navigation, read by the layout
  error.rs       errors, answered with a rendered error page
templates/
  _layout.html   the skeleton every page extends
  _macros.html   markup shared between pages
  home.html, books.html, book.html, book_form.html, about.html, error.html
static/
  htmx.min.js    htmx 2.0.10, vendored (0BSD, see htmx.LICENSE)
  style.css      plain CSS
```

## The patterns

**One layout.** Each page template starts with `{% extends "_layout.html" %}` and fills in
its `title` and `content` blocks. Each page's struct has a `current_path` field, which the
layout compares with the navigation in `src/nav.rs` to mark the current link.

**`hx-boost` on `<body>`.** htmx takes over every link and form: it fetches the next page in
the background and swaps in its `<body>`, keeping the address bar and history in step. The
server does nothing special for these; it sends the whole page as usual.

**Part of a page, from the same template.** The search box sends
`hx-get="/books" hx-target="#results"`. htmx adds `HX-Request: true` and `HX-Target: results`
to the request, and the server answers with only the `{% block results %}` of `books.html`.
Declaring `#[template(path = "books.html", blocks = ["results"])]` makes askama generate
`as_results()` for this, so the markup is never written twice. The response says
`Vary: HX-Request, HX-Target`, so a cache never mixes the two up.

**A form that fixes itself in place.** The add form posts with
`hx-post … hx-target="this" hx-swap="outerHTML"`. If something is missing, the server sends
back just the form block with what was typed and an error under each wrong field, and status
200, because htmx 2 does not swap 4xx answers by default. On success it sends an
`HX-Location` header and htmx goes to the new book's page. Without JavaScript the same form
gets the whole page back (status 422) or a `303` redirect.

**Removing a row.** Each row's Remove button is a little form with
`hx-post … hx-target="closest li" hx-swap="outerHTML" hx-confirm="…"`: htmx asks, posts, and
replaces the row with the empty answer. Without JavaScript the server redirects back to the
list.

**Telling the requests apart.** `wants_fragment()` in `src/routes.rs` is the one check:
`HX-Request: true` without `HX-Boosted: true`. A boosted request, or one with no htmx at all,
wants the whole page.

**Escaping.** askama escapes HTML in `.html` templates unless a value is marked `|safe`, so
whatever is typed into the form is shown as text.

## Taking it further

- Keep the data in a database: swap `Library` for queries with
  [sqlx](https://github.com/launchbadge/sqlx), and keep the handlers as they are.
- Styles: the [Tailwind CSS standalone CLI](https://tailwindcss.com/blog/standalone-cli) needs
  no Node.js and can watch `templates/` for class names.
- Reload in development: `cargo watch -x run` restarts the server when a file changes.
