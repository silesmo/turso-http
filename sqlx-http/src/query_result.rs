#[derive(Debug, Default, Clone)]
pub struct HttpQueryResult {
    pub(crate) rows_affected: u64,
}

impl HttpQueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }
}

impl Extend<HttpQueryResult> for HttpQueryResult {
    fn extend<T: IntoIterator<Item = HttpQueryResult>>(&mut self, iter: T) {
        for item in iter {
            self.rows_affected += item.rows_affected;
        }
    }
}
