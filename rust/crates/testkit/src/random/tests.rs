use super::*;

#[test]
fn the_first_numbers_of_seed_one_are_those_of_the_shifts_13_7_17() {
    let mut random = Random::new(1);
    assert_eq!(random.next_u64(), 0x4082_2041);
    assert_eq!(random.next_u64(), 0x1000_4106_0C01_1441);
    assert_eq!(random.next_u64(), 0x9B1E_842F_6E86_2629);
}

#[test]
fn the_same_seed_gives_the_same_numbers() {
    for seed in [0, 1, 42, u64::MAX] {
        let mut first = Random::new(seed);
        let mut second = Random::new(seed);
        let first_numbers: Vec<u64> = (0..100).map(|_| first.next_u64()).collect();
        let second_numbers: Vec<u64> = (0..100).map(|_| second.next_u64()).collect();
        assert_eq!(first_numbers, second_numbers);
        assert!(first_numbers.iter().all(|number| *number != 0));
        assert_eq!(first.seed(), seed);
    }
}

#[test]
fn different_seeds_give_different_numbers() {
    assert_ne!(Random::new(1).next_u64(), Random::new(2).next_u64());
}

#[test]
fn below_stays_below_the_bound_and_reaches_every_number() {
    let mut random = Random::new(7);
    let mut seen = [false; 6];
    for _ in 0..1000 {
        let number = random.below(6);
        assert!(number < 6);
        seen[number] = true;
    }
    assert!(seen.iter().all(|was_seen| *was_seen));
    assert_eq!(random.below(1), 0);
}

#[test]
fn pick_picks_every_item_and_chance_follows_its_odds() {
    let mut random = Random::new(11);
    let items = ["one", "two", "three"];
    let mut counts = [0; 3];
    for _ in 0..3000 {
        let picked = random.pick(&items);
        let position = items
            .iter()
            .position(|item| item == picked)
            .expect("the item is one of them");
        counts[position] += 1;
    }
    assert!(counts.iter().all(|count| (800..1200).contains(count)), "{counts:?}");
    let happened = (0..4000).filter(|_| random.chance(1, 4)).count();
    assert!((800..1200).contains(&happened), "{happened}");
    assert!((0..100).all(|_| !random.chance(0, 3)));
    assert!((0..100).all(|_| random.chance(3, 3)));
}

#[test]
#[should_panic(expected = "nothing to pick from")]
fn picking_from_nothing_fails() {
    let mut random = Random::new(3);
    let empty: [u8; 0] = [];
    let _ = random.pick(&empty);
}
