pub struct Allergies {
    score: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Allergen {
    Eggs = 1,
    Peanuts = 2,
    Shellfish = 4,
    Strawberries = 8,
    Tomatoes = 16,
    Chocolate = 32,
    Pollen = 64,
    Cats = 128,
}

const ALLERGENS: [Allergen; 8] = [
    Allergen::Eggs,
    Allergen::Peanuts,
    Allergen::Shellfish,
    Allergen::Strawberries,
    Allergen::Tomatoes,
    Allergen::Chocolate,
    Allergen::Pollen,
    Allergen::Cats,
];

impl Allergies {
    pub fn new(score: u32) -> Self {
        Self { score }
    }

    pub fn is_allergic_to(&self, allergen: &Allergen) -> bool {
        self.score & *allergen as u32 == *allergen as u32
    }

    pub fn allergies(&self) -> Vec<Allergen> {
        ALLERGENS
            .iter()
            .filter(|a| self.is_allergic_to(a))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::allergies::*;
    fn compare_allergy_vectors(expected: &[Allergen], actual: &[Allergen]) {
        for element in expected {
            if !actual.contains(element) {
                panic!("Allergen missing\n  {element:?} should be in {actual:?}");
            }
        }
        if actual.len() != expected.len() {
            panic!(
            "Allergy vectors are of different lengths\n  expected {expected:?}\n  got {actual:?}"
        );
        }
    }
    #[test]
    fn is_not_allergic_to_anything() {
        let allergies = Allergies::new(0);
        assert!(!allergies.is_allergic_to(&Allergen::Peanuts));
        assert!(!allergies.is_allergic_to(&Allergen::Cats));
        assert!(!allergies.is_allergic_to(&Allergen::Strawberries));
    }
    #[test]
    fn is_allergic_to_eggs() {
        assert!(Allergies::new(1).is_allergic_to(&Allergen::Eggs));
    }
    #[test]
    fn is_allergic_to_eggs_and_shellfish_but_not_strawberries() {
        let allergies = Allergies::new(5);
        assert!(allergies.is_allergic_to(&Allergen::Eggs));
        assert!(allergies.is_allergic_to(&Allergen::Shellfish));
        assert!(!allergies.is_allergic_to(&Allergen::Strawberries));
    }
    #[test]
    fn no_allergies_at_all() {
        let expected = &[];
        let allergies = Allergies::new(0).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_just_eggs() {
        let expected = &[Allergen::Eggs];
        let allergies = Allergies::new(1).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_just_peanuts() {
        let expected = &[Allergen::Peanuts];
        let allergies = Allergies::new(2).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_just_strawberries() {
        let expected = &[Allergen::Strawberries];
        let allergies = Allergies::new(8).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_eggs_and_peanuts() {
        let expected = &[Allergen::Eggs, Allergen::Peanuts];
        let allergies = Allergies::new(3).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_eggs_and_shellfish() {
        let expected = &[Allergen::Eggs, Allergen::Shellfish];
        let allergies = Allergies::new(5).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_many_things() {
        let expected = &[
            Allergen::Strawberries,
            Allergen::Tomatoes,
            Allergen::Chocolate,
            Allergen::Pollen,
            Allergen::Cats,
        ];
        let allergies = Allergies::new(248).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn allergic_to_everything() {
        let expected = &[
            Allergen::Eggs,
            Allergen::Peanuts,
            Allergen::Shellfish,
            Allergen::Strawberries,
            Allergen::Tomatoes,
            Allergen::Chocolate,
            Allergen::Pollen,
            Allergen::Cats,
        ];
        let allergies = Allergies::new(255).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
    #[test]
    fn scores_over_255_do_not_trigger_false_positives() {
        let expected = &[
            Allergen::Eggs,
            Allergen::Shellfish,
            Allergen::Strawberries,
            Allergen::Tomatoes,
            Allergen::Chocolate,
            Allergen::Pollen,
            Allergen::Cats,
        ];
        let allergies = Allergies::new(509).allergies();
        compare_allergy_vectors(expected, &allergies);
    }
}
