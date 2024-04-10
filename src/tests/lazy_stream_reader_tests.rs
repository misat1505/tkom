#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::lazy_stream_reader::ILazyStreamReader;
    use crate::lazy_stream_reader::LazyStreamReader;
    use crate::lazy_stream_reader::{ETX, STX};

    #[test]
    fn test_lazy_stream_reader() {
        let code = BufReader::new(
            r#"hello
world"#
                .as_bytes(),
        );
        let mut stream_reader = LazyStreamReader::new(code);

        let expected: Vec<(char, u32, u32)> = vec![
            ('h', 1, 1),
            ('e', 1, 2),
            ('l', 1, 3),
            ('l', 1, 4),
            ('o', 1, 5),
            ('\n', 1, 6),
            ('w', 2, 1),
            ('o', 2, 2),
            ('r', 2, 3),
            ('l', 2, 4),
            ('d', 2, 5),
            (ETX, 2, 6),
            (ETX, 2, 6),
        ];

        assert_eq!(*stream_reader.current(), STX);
        assert_eq!(stream_reader.position().line, 0);
        assert_eq!(stream_reader.position().column, 0);

        for (exp_char, exp_line, exp_col) in &expected {
            assert_eq!(*stream_reader.next().unwrap(), *exp_char);
            assert_eq!(stream_reader.position().line, *exp_line);
            assert_eq!(stream_reader.position().column, *exp_col);
        }
    }
}
