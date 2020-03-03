extern crate hypercore;
extern crate rand;

use hypercore::bitfield::{Bitfield, Change::*};
use rand::Rng;

#[test]
fn set_and_get() {
    let mut b = Bitfield::new();

    assert_eq!(b.get(0), false);
    assert_eq!(b.set(0, true), Changed);
    assert_eq!(b.set(0, true), Unchanged);
    assert_eq!(b.get(0), true);

    assert_eq!(b.get(1_424_244), false);
    assert_eq!(b.set(1_424_244, true), Changed);
    assert_eq!(b.set(1_424_244, true), Unchanged);
    assert_eq!(b.get(1_424_244), true);
}

#[test]
fn set_and_get_tree() {
    let mut b = Bitfield::new();

    {
        let tree = &mut b.tree;

        assert_eq!(tree.get(0), false);
        assert_eq!(tree.set(0, true), Changed);
        assert_eq!(tree.set(0, true), Unchanged);
        assert_eq!(tree.get(0), true);

        assert_eq!(tree.get(1_424_244), false);
        assert_eq!(tree.set(1_424_244, true), Changed);
        assert_eq!(tree.set(1_424_244, true), Unchanged);
        assert_eq!(tree.get(1_424_244), true);
    }

    assert_eq!(b.get(0), false);
    assert_eq!(b.get(1_424_244), false);
}

#[test]
fn set_and_index() {
    let mut b = Bitfield::new();

    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.next(), Some(0));
    }

    b.set(0, true);
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(1));
    }

    b.set(479, true);
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(478).next(), Some(478));
        assert_eq!(iter.next(), Some(480));
    }

    b.set(1, true);
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(2));
    }

    b.set(2, true);
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(3));
    }

    b.set(3, true);
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(4));
    }

    let len = b.len();
    for i in 0..len {
        b.set(i, true);
    }
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(len));
    }

    for i in 0..len {
        b.set(i, false);
    }
    {
        let mut iter = b.iterator_with_range(0, 100_000_000);
        assert_eq!(iter.seek(0).next(), Some(0));
    }
}

#[test]
fn set_and_index_random() {
    let mut b = Bitfield::new();

    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        assert!(check(&mut b), "index validates");
        set(&mut b, rng.gen_range(0, 2000), rng.gen_range(0, 8));
    }

    assert!(check(&mut b), "index validates");

    fn check(b: &mut Bitfield) -> bool {
        let mut all = vec![true; b.len() as usize];

        {
            let mut iter = b.iterator();

            while let Some(i) = iter.next() {
                all[i as usize] = false;
            }
        }

        for (i, &v) in all.iter().enumerate() {
            if b.get(i as u64) != v {
                return false;
            }
        }

        true
    }

    fn set(b: &mut Bitfield, i: u64, n: u64) {
        for j in i..i + n {
            b.set(j, true);
        }
    }
}

#[test]
fn get_total_positive_bits() {
    let mut b = Bitfield::new();

    assert_eq!(b.set(1, true), Changed);
    assert_eq!(b.set(2, true), Changed);
    assert_eq!(b.set(4, true), Changed);
    assert_eq!(b.set(5, true), Changed);
    assert_eq!(b.set(39, true), Changed);

    assert_eq!(b.total_with_range(0..4), 2);
    assert_eq!(b.total_with_range(3..4), 0);
    assert_eq!(b.total_with_range(3..5), 1);
    assert_eq!(b.total_with_range(3..40), 3);
    assert_eq!(b.total(), 5);
    assert_eq!(b.total_with_start(7), 1);
}

#[test]
fn bitfield_dedup() {
    let mut b = Bitfield::new();

    for i in 0..32 * 1024 {
        b.set(i, true);
    }

    for i in 0..64 * 1024 {
        b.tree.set(i, true);
    }

    assert!(b.get(8 * 1024));
    assert!(b.get(16 * 1024));
    b.set(8 * 1024, false);
    assert!(!b.get(8 * 1024));
    assert!(b.get(16 * 1024));
}

#[test]
fn bitfield_compress() {
    let mut b = Bitfield::new();
    assert_eq!(b.compress(0, 0).unwrap(), vec![0]);

    b.set(1, true);
    assert_eq!(b.compress(0, 0).unwrap(), vec![2, 64, 253, 31]);

    b.set(1_424_244, true);
    assert_eq!(
        b.compress(0, 0).unwrap(),
        vec![2, 64, 181, 187, 43, 2, 8, 197, 4]
    );
    assert_eq!(b.compress(0, 1).unwrap(), vec![2, 64, 253, 31]);
    assert_eq!(
        b.compress(1_424_244, 1).unwrap(),
        vec![185, 27, 2, 8, 197, 4]
    );

    assert_eq!(b.compress(1_424_244_000, 1).unwrap(), vec![0]);
}
