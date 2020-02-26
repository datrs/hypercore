use std::iter;

#[derive(Debug)]
struct Book {
    pub title: String,
}

#[derive(Debug)]
struct BookShelf {
    pub books: Vec<Book>,
}

#[derive(Debug)]
struct BookShelfIterator<'b> {
    /// Keeps track which index we're currently at.
    pub cursor: u64,
    /// Borrow of the Bookshelf we're going to iterate over.
    pub inner: &'b BookShelf,
}

impl BookShelf {
    /// Return an iterator over all values.
    pub fn iter(&self) -> BookShelfIterator<'_> {
        BookShelfIterator {
            inner: self,
            cursor: 0,
        }
    }
}

impl<'b> iter::Iterator for BookShelfIterator<'b> {
    type Item = &'b Book;

    fn next(&mut self) -> Option<Self::Item> {
        let cursor = self.cursor;
        self.cursor += 1;

        if cursor >= self.inner.books.len() as u64 {
            None
        } else {
            Some(&self.inner.books[cursor as usize])
        }
    }
}

impl<'b> iter::IntoIterator for &'b BookShelf {
    type Item = &'b Book;
    type IntoIter = BookShelfIterator<'b>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            cursor: 0,
            inner: self,
        }
    }
}

fn main() {
    let library = BookShelf {
        books: vec![
            Book {
                title: "Das Kapital I".into(),
            },
            Book {
                title: "Das Kapital II".into(),
            },
            Book {
                title: "Das Kapital III".into(),
            },
        ],
    };

    for book in library.iter() {
        println!("book {}", book.title);
    }

    for book in &library {
        println!("book {}", book.title);
    }
}
