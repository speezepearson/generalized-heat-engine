use std::{fmt::Display, iter::Sum, mem::swap};

use rand::{seq::SliceRandom, RngCore, SeedableRng};

const BATTERY_SIZE: usize = 20;
const BATH_SIZE: usize = 200;
const N_STEPS: u64 = 1000000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct World {
    t: i64,
    battery: Vec<bool>,
    hot_bath: Vec<bool>,
    cold_bath: Vec<bool>,
}

fn sumbools(xs: &Vec<bool>) -> usize {
    xs.iter().map(|&x| x as usize).sum()
}

impl Display for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{t:>width$} [({h:>3}/{l:>3})",
            t = self.t,
            width = 5,
            h = sumbools(&self.battery),
            l = self.battery.len()
        )?;
        // for i in 0..BATTERY_SIZE {
        //     write!(f, "{}", if self.battery[i] { "#" } else { " " })?;
        // }
        write!(
            f,
            "] [({h:>3}/{l:>3})",
            h = sumbools(&self.hot_bath),
            l = self.hot_bath.len()
        )?;
        // for i in 0..self.hot_bath.len() {
        //     write!(f, "{}", if self.hot_bath[i] { "#" } else { " " })?;
        // }
        write!(
            f,
            "] [({h:>3}/{l:>3})",
            h = sumbools(&self.cold_bath),
            l = self.cold_bath.len()
        )?;
        // for i in 0..self.cold_bath.len() {
        //     write!(f, "{}", if self.cold_bath[i] { "#" } else { " " })?;
        // }
        write!(f, "]")?;
        Ok(())
    }
}

trait Rule {
    fn step(&self, world: &mut World);
    fn inverse(&self) -> Box<dyn Rule>;
}

#[derive(Clone, Copy, Debug)]
struct ProbeAndSwap;

impl Rule for ProbeAndSwap {
    fn step(&self, world: &mut World) {
        if world.hot_bath[0] {
            swap(&mut world.battery[1], &mut world.hot_bath[1]);
        }
    }

    fn inverse(&self) -> Box<dyn Rule> {
        Box::new(ProbeAndSwap)
    }
}

#[derive(Clone, Debug)]
struct Permute {
    battery: Vec<usize>,
    hot_bath: Vec<usize>,
    cold_bath: Vec<usize>,
}

impl Rule for Permute {
    fn step(&self, world: &mut World) {
        permute(&self.battery, &mut world.battery);
        permute(&self.hot_bath, &mut world.hot_bath);
        permute(&self.cold_bath, &mut world.cold_bath);
    }

    fn inverse(&self) -> Box<dyn Rule> {
        let mut inverse = self.clone();
        for i in 0..self.battery.len() {
            inverse.battery[self.battery[i]] = i;
        }
        for i in 0..self.hot_bath.len() {
            inverse.hot_bath[self.hot_bath[i]] = i;
        }
        for i in 0..self.cold_bath.len() {
            inverse.cold_bath[self.cold_bath[i]] = i;
        }
        Box::new(inverse)
    }
}

#[derive(Clone, Debug)]
struct WeirdPermute {
    seed: u64,
    inverted: bool,
}

impl Rule for WeirdPermute {
    fn step(&self, world: &mut World) {
        let t = world.t - if self.inverted { 1 } else { 0 };
        for target in [
            &mut world.battery,
            &mut world.hot_bath,
            &mut world.cold_bath,
        ] {
            let mut perm =
                generate_random_permutation(target.len(), self.seed.wrapping_add_signed(t));
            if self.inverted {
                perm = invert_permutation(&perm);
            }
            permute(&perm, target);
        }
    }

    fn inverse(&self) -> Box<dyn Rule> {
        Box::new(WeirdPermute {
            seed: self.seed,
            inverted: !self.inverted,
        })
    }
}

#[cfg(test)]
mod test {

    mod weird_conditional_permute {
        use crate::*;
        #[test]
        fn test_inverse() {
            let mut world = World {
                t: 0,
                battery: [
                    false, true, false, true, false, true, false, true, false, true,
                ]
                .to_vec(),
                hot_bath: [
                    true, false, true, false, true, false, true, false, true, false,
                ]
                .to_vec(),
                cold_bath: [false; 10].to_vec(),
            };
            let permute = WeirdPermute {
                seed: 0,
                inverted: false,
            };
            permute.step(&mut world);
            world.t += 1;
            permute.inverse().step(&mut world);
            world.t -= 1;
            assert_eq!(
                world,
                World {
                    t: 0,
                    battery: [false, true, false, true, false, true, false, true, false, true,]
                        .to_vec(),
                    hot_bath: [true, false, true, false, true, false, true, false, true, false,]
                        .to_vec(),
                    cold_bath: [false; 10].to_vec(),
                }
            );
        }
    }
}

#[derive(Clone, Debug)]
struct CondSwap;

impl Rule for CondSwap {
    fn step(&self, world: &mut World) {
        let h = &mut world.hot_bath;
        let c = &mut world.cold_bath;
        let b = &mut world.battery;
        if (h[0], h[1], c[0], b[0]) == (true, true, false, false)
            || (h[0], h[1], c[0], b[0]) == (false, false, true, true)
        {
            swap(&mut h[0], &mut c[0]);
            swap(&mut h[1], &mut b[0]);
        }
    }

    fn inverse(&self) -> Box<dyn Rule> {
        Box::new(Self)
    }
}

impl Rule for Vec<Box<dyn Rule>> {
    fn step(&self, world: &mut World) {
        for rule in self {
            rule.step(world);
        }
    }

    fn inverse(&self) -> Box<dyn Rule> {
        let mut inverse = Vec::new();
        for rule in self.iter().rev() {
            inverse.push(rule.inverse());
        }
        Box::new(inverse)
    }
}

fn generate_random_permutation(n: usize, seed: u64) -> Vec<usize> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut permutation = (0..n).collect::<Vec<usize>>();
    permutation.shuffle(&mut rng);
    permutation
}

fn permute<T: Clone, S: AsMut<[T]>>(permutation: &Vec<usize>, xs: &mut S) {
    let buf = xs.as_mut();
    let orig = buf.to_vec();
    for i in 0..buf.len() {
        buf[i] = orig[permutation[i]].clone();
    }
}

fn invert_permutation(permutation: &Vec<usize>) -> Vec<usize> {
    let mut inverse = vec![0; permutation.len()];
    for i in 0..permutation.len() {
        inverse[permutation[i]] = i;
    }
    inverse
}

#[cfg(test)]
mod test_invert_permutation {
    use crate::*;
    #[test]
    fn test_invert_permutation() {
        let mut xs = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let permutation = generate_random_permutation(xs.len(), 0);
        permute(&permutation, &mut xs);
        permute(&invert_permutation(&permutation), &mut xs);
        assert_eq!(xs, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}

fn main() {
    // return;
    let mut world = World {
        t: 0,
        battery: [false; BATTERY_SIZE].to_vec(),
        hot_bath: [[true; BATH_SIZE / 2], [false; BATH_SIZE / 2]]
            .to_vec()
            .concat(),
        cold_bath: [false; BATH_SIZE].to_vec(),
    };
    let mut revworld = world.clone();

    let permutation = WeirdPermute {
        seed: rand::thread_rng().next_u64(),
        inverted: false,
    };

    let rules: Vec<Box<dyn Rule>> = vec![Box::new(CondSwap), Box::new(permutation)];
    let inv_rules = rules.inverse();

    println!("{world} ");
    for _ in 0..N_STEPS {
        rules.step(&mut world);
        world.t += 1;

        inv_rules.step(&mut revworld);
        revworld.t -= 1;

        if world.t > 0 && is_pow2(world.t as u64) {
            println!("{world} ");
        }
    }

    // println!("\n\n\n");

    // println!("{world}  ---  {revworld}");
    // for _ in 0..N_STEPS {
    //     inv_rules.step(&mut world);
    //     world.t -= 1;

    //     rules.step(&mut revworld);
    //     revworld.t += 1;

    //     println!("{world}  ---  {revworld}");
    // }
}

fn is_pow2(n: u64) -> bool {
    n & (n - 1) == 0
}
