use rand::{seq::SliceRandom, thread_rng};

const ENGLISH_WORDS: &str = include_str!("../assets/english.txt");

pub fn generate(count: usize) -> Vec<String> {
    let source: Vec<&str> = ENGLISH_WORDS
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect();
    let mut rng = thread_rng();
    let mut output = Vec::with_capacity(count);

    while output.len() < count {
        let mut batch = source.clone();
        batch.shuffle(&mut rng);
        for word in batch {
            if output.last().is_some_and(|last| last == word) {
                continue;
            }
            output.push(word.to_owned());
            if output.len() == count {
                break;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_requested_count_without_adjacent_duplicates() {
        let words = generate(2_000);
        assert_eq!(words.len(), 2_000);
        assert!(words.windows(2).all(|pair| pair[0] != pair[1]));
    }
}
