use std::hash::{Hash, Hasher};

use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
    Serializer,
};

#[derive(Debug)]
pub struct CemantixWord {
    pub word: String,
    pub rank: isize,
    pub score: f32,
}

impl CemantixWord {
    pub fn new(word: String, rank: isize, score: f32) -> Self {
        Self { word, rank, score }
    }
}

impl PartialEq for CemantixWord {
    fn eq(&self, other: &Self) -> bool {
        self.word == other.word
    }
}

impl Eq for CemantixWord {}

impl Hash for CemantixWord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.word.hash(state);
    }
}

impl<'de> serde::de::Deserialize<'de> for CemantixWord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CemantixWordVisitor;
        impl<'de> Visitor<'de> for CemantixWordVisitor {
            type Value = CemantixWord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct CemantixWord")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let word = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let rank = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let score = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                Ok(CemantixWord::new(word, rank, score))
            }
        }

        const FIELDS: &[&str] = &["word", "rank", "score"];
        deserializer.deserialize_struct("CemantixWord", FIELDS, CemantixWordVisitor)
    }
}

impl serde::Serialize for CemantixWord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_seq(Some(3))?;
        state.serialize_element(&self.word)?;
        state.serialize_element(&self.rank)?;
        state.serialize_element(&self.score)?;
        state.end()
    }
}
