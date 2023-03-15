pub struct CodonsInfo<'a> {
    // We fake using 'a here, so the compiler does not complain that
    // "parameter `'a` is never used". Delete when no longer needed.
    phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> CodonsInfo<'a> {
    pub fn name_for(&self, codon: &str) -> Option<&'a str> {
        match codon {
            "AUG" => Some("Methionine"),
            "UUU" | "UUC" => Some("Phenylalanine"),
            "UUA" | "UUG" => Some("Leucine"),
            "UCU" | "UCC" | "UCA" | "UCG" => Some("Serine"),
            "UAU" | "UAC" => Some("Tyrosine"),
            "UGU" | "UGC" => Some("Cysteine"),
            "UGG" => Some("Tryptophan"),
            "UAA" | "UAG" | "UGA" => Some("STOP"),
            _ => None,
        }
    }

    pub fn of_rna(&self, rna: &str) -> Option<Vec<&'a str>> {
        unimplemented!("Return a list of protein names that correspond to the '{rna}' RNA string or None if the RNA string is invalid");
    }
}

pub fn parse<'a>(pairs: Vec<(&'a str, &'a str)>) -> CodonsInfo<'a> {
    unimplemented!("Construct a new CodonsInfo struct from given pairs: {pairs:?}");
}

#[cfg(test)]
mod tests {
    use crate::protein_translation as proteins;

    #[test]
    fn test_methionine() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("AUG"), Some("methionine"));
    }
    #[test]
    fn test_cysteine_tgt() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("UGU"), Some("cysteine"));
    }
    #[test]
    #[ignore]
    fn test_stop() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("UAA"), Some("stop codon"));
    }
    #[test]
    #[ignore]
    fn test_valine() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("GUU"), Some("valine"));
    }
    #[test]
    #[ignore]
    fn test_isoleucine() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("AUU"), Some("isoleucine"));
    }
    #[test]
    #[ignore]
    fn test_arginine_name() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.name_for("CGA"), Some("arginine"));
        assert_eq!(info.name_for("AGA"), Some("arginine"));
        assert_eq!(info.name_for("AGG"), Some("arginine"));
    }
    #[test]
    #[ignore]
    fn empty_is_invalid() {
        let info = proteins::parse(make_pairs());
        assert!(info.name_for("").is_none());
    }
    #[test]
    #[ignore]
    fn x_is_not_shorthand_so_is_invalid() {
        let info = proteins::parse(make_pairs());
        assert!(info.name_for("VWX").is_none());
    }
    #[test]
    #[ignore]
    fn too_short_is_invalid() {
        let info = proteins::parse(make_pairs());
        assert!(info.name_for("AU").is_none());
    }
    #[test]
    #[ignore]
    fn too_long_is_invalid() {
        let info = proteins::parse(make_pairs());
        assert!(info.name_for("ATTA").is_none());
    }
    #[test]
    #[ignore]
    fn test_translates_rna_strand_into_correct_protein() {
        let info = proteins::parse(make_pairs());
        assert_eq!(
            info.of_rna("AUGUUUUGG"),
            Some(vec!["methionine", "phenylalanine", "tryptophan"])
        );
    }
    #[test]
    #[ignore]
    fn test_stops_translation_if_stop_codon_present() {
        let info = proteins::parse(make_pairs());
        assert_eq!(
            info.of_rna("AUGUUUUAA"),
            Some(vec!["methionine", "phenylalanine"])
        );
    }
    #[test]
    #[ignore]
    fn test_stops_translation_of_longer_strand() {
        let info = proteins::parse(make_pairs());
        assert_eq!(
            info.of_rna("UGGUGUUAUUAAUGGUUU"),
            Some(vec!["tryptophan", "cysteine", "tyrosine"])
        );
    }
    #[test]
    #[ignore]
    fn test_invalid_codons() {
        let info = proteins::parse(make_pairs());
        assert!(info.of_rna("CARROT").is_none());
    }
    #[test]
    #[ignore]
    fn test_invalid_length() {
        let info = proteins::parse(make_pairs());
        assert!(info.of_rna("AUGUA").is_none());
    }
    #[test]
    #[ignore]
    fn test_valid_stopped_rna() {
        let info = proteins::parse(make_pairs());
        assert_eq!(info.of_rna("AUGUAAASDF"), Some(vec!["methionine"]));
    }
    // The input data constructor. Returns a list of codon, name pairs.
    fn make_pairs() -> Vec<(&'static str, &'static str)> {
        let grouped = vec![
            ("isoleucine", vec!["AUU", "AUC", "AUA"]),
            ("valine", vec!["GUU", "GUC", "GUA", "GUG"]),
            ("phenylalanine", vec!["UUU", "UUC"]),
            ("methionine", vec!["AUG"]),
            ("cysteine", vec!["UGU", "UGC"]),
            ("alanine", vec!["GCU", "GCC", "GCA", "GCG"]),
            ("glycine", vec!["GGU", "GGC", "GGA", "GGG"]),
            ("proline", vec!["CCU", "CCC", "CCA", "CCG"]),
            ("threonine", vec!["ACU", "ACC", "ACA", "ACG"]),
            ("serine", vec!["AGU", "AGC"]),
            ("tyrosine", vec!["UAU", "UAC"]),
            ("tryptophan", vec!["UGG"]),
            ("glutamine", vec!["CAA", "CAG"]),
            ("asparagine", vec!["AAU", "AAC"]),
            ("histidine", vec!["CAU", "CAC"]),
            ("glutamic acid", vec!["GAA", "GAG"]),
            ("aspartic acid", vec!["GAU", "GAC"]),
            ("lysine", vec!["AAA", "AAG"]),
            ("arginine", vec!["CGU", "CGC", "CGA", "CGG", "AGA", "AGG"]),
            ("stop codon", vec!["UAA", "UAG", "UGA"]),
        ];
        let mut pairs = Vec::<(&'static str, &'static str)>::new();
        for (name, codons) in grouped.into_iter() {
            for codon in codons {
                pairs.push((codon, name));
            }
        }
        pairs.sort_by(|&(_, a), &(_, b)| a.cmp(b));
        pairs
    }
}
