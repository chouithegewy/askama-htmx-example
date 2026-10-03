//! The site's name and navigation, defined once and read by "templates/_layout.html", so
//! adding a section does not mean editing a template.

pub const SITE: &str = "Reading List";

pub struct Link {
    pub path: &'static str,
    pub label: &'static str,
}

pub const NAV: &[Link] = &[
    Link {
        path: "/",
        label: "Home",
    },
    Link {
        path: "/books",
        label: "Books",
    },
    Link {
        path: "/books/new",
        label: "Add a book",
    },
    Link {
        path: "/about",
        label: "About",
    },
];
