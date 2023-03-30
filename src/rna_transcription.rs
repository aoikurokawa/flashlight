const FOUR_DNA: [char; 4] = ['G', 'C', 'T', 'A'];
const FOUR_RNA: [char; 4] = ['C', 'G', 'A', 'U'];

#[derive(Debug, PartialEq, Eq)]
pub struct Dna(String);

#[derive(Debug, PartialEq, Eq)]
pub struct Rna(String);

impl Dna {
    pub fn new(dna: &str) -> Result<Dna, usize> {
        for (index, dna_ch) in dna.chars().enumerate() {
            if !FOUR_DNA.contains(&dna_ch) {
                return Err(index);
            }
        }
        Ok(Dna(dna.to_string()))
    }

    pub fn into_rna(self) -> Rna {
        let mut rna = "".to_string();
        for dna_ch in self.0.chars() {
            match dna_ch {
                'G' => rna = format!("{}{}", rna, "C"),
                'C' => rna = format!("{}{}", rna, "G"),
                'T' => rna = format!("{}{}", rna, "A"),
                'A' => rna = format!("{}{}", rna, "U"),
                _ => rna = format!(""),
            }
        }
        Rna(rna)
    }
}

impl Rna {
    pub fn new(rna: &str) -> Result<Rna, usize> {
        for (index, rna_ch) in rna.chars().enumerate() {
            if !FOUR_RNA.contains(&rna_ch) {
                return Err(index);
            }
        }
        Ok(Rna(rna.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::rna_transcription as dna;
    #[test]
    fn test_valid_dna_input() {
        assert!(dna::Dna::new("GCTA").is_ok());
    }
    #[test]
    fn test_valid_rna_input() {
        assert!(dna::Rna::new("CGAU").is_ok());
    }
    #[test]
    fn test_invalid_dna_input() {
        // Invalid character
        assert_eq!(dna::Dna::new("X").err(), Some(0));
        // Valid nucleotide, but invalid in context
        assert_eq!(dna::Dna::new("U").err(), Some(0));
        // Longer string with contained errors
        assert_eq!(dna::Dna::new("ACGTUXXCTTAA").err(), Some(4));
    }
    #[test]
    fn test_invalid_rna_input() {
        // Invalid character
        assert_eq!(dna::Rna::new("X").unwrap_err(), 0);
        // Valid nucleotide, but invalid in context
        assert_eq!(dna::Rna::new("T").unwrap_err(), 0);
        // Longer string with contained errors
        assert_eq!(dna::Rna::new("ACGUTTXCUUAA").unwrap_err(), 4);
    }
    #[test]
    fn test_acid_equals_acid() {
        assert_eq!(dna::Dna::new("CGA").unwrap(), dna::Dna::new("CGA").unwrap());
        assert_ne!(dna::Dna::new("CGA").unwrap(), dna::Dna::new("AGC").unwrap());
        assert_eq!(dna::Rna::new("CGA").unwrap(), dna::Rna::new("CGA").unwrap());
        assert_ne!(dna::Rna::new("CGA").unwrap(), dna::Rna::new("AGC").unwrap());
    }
    #[test]
    fn test_transcribes_cytosine_guanine() {
        assert_eq!(
            dna::Rna::new("G").unwrap(),
            dna::Dna::new("C").unwrap().into_rna()
        );
    }
    #[test]
    fn test_transcribes_guanine_cytosine() {
        assert_eq!(
            dna::Rna::new("C").unwrap(),
            dna::Dna::new("G").unwrap().into_rna()
        );
    }
    #[test]
    fn test_transcribes_adenine_uracil() {
        assert_eq!(
            dna::Rna::new("U").unwrap(),
            dna::Dna::new("A").unwrap().into_rna()
        );
    }
    #[test]
    fn test_transcribes_thymine_to_adenine() {
        assert_eq!(
            dna::Rna::new("A").unwrap(),
            dna::Dna::new("T").unwrap().into_rna()
        );
    }
    #[test]
    fn test_transcribes_all_dna_to_rna() {
        assert_eq!(
            dna::Rna::new("UGCACCAGAAUU").unwrap(),
            dna::Dna::new("ACGTGGTCTTAA").unwrap().into_rna()
        )
    }
}
