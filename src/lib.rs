pub mod cemantix_word;
pub mod utils;
pub mod words_getter;
pub mod options {
    pub mod extend;
    pub mod graph;
    pub mod nearby;
    pub mod options;
    pub mod remove_useless_words;
    pub mod solve;
    pub mod sort;
}
pub const HISTORY_FORMAT: &str = "%d-%m-%Y";

#[cfg(test)]
mod test {
    use std::cmp::Ordering;

    use crate::cemantix_word::CemantixWord;

    #[test]
    fn cemantix_word_order() {
        let w1 = CemantixWord::new(String::new(), 0, 0.52);
        let w2 = CemantixWord::new(String::new(), 0, 0.23);

        assert_eq!(w1.cmp(&w2), Ordering::Greater);

        assert!(w1 > w2);
        assert!(w1 >= w2);
        assert!(w2 <= w1);
        assert!(w2 < w1);
    }

    #[test]
    fn cemantix_word_eq() {
        let s1 = String::from("coucou");
        let s2 = String::from("lsdkfj");
        assert_ne!(s1, s2);
        let w1 = CemantixWord::new(s1, 0, 0.0);
        let w2 = CemantixWord::new(s2, 0, 0.0);

        assert_ne!(w1, w2)
    }
}
