/// Formatter helps format key-value data in a way that makes it easy
/// to modify the data and read it back in.
pub struct KeyValueFormatter {
    pub key_padding: usize,
    pub single_delimiter: String,
    pub multi_delimiter: String,
    pub value_delimiter: String,
}

impl KeyValueFormatter {
    pub fn new() -> Self {
        Self {
            key_padding: 60,
            single_delimiter: " = ".into(),
            multi_delimiter: " : ".into(),
            value_delimiter: ", ".into(),
        }
    }

    pub fn format_single(&self, key: &str, value: &str) -> String {
        format!(
            "{:width$}{}{}",
            key,
            self.single_delimiter,
            value,
            width = self.key_padding,
        )
    }

    pub fn format_multi<'a, 'b, 'c>(
        &'a self,
        key: &'b str,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> String {
        format!(
            "{:width$}{}{}",
            key,
            self.multi_delimiter,
            values
                .into_iter()
                .map(|x| x.into())
                .collect::<Vec<_>>()
                .join(&self.value_delimiter),
            width = self.key_padding,
        )
    }
}
