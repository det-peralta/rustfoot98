//! Borland Delphi 1 `System.Random`, verified from `10c0:218a` (seed update) and `10c0:212d` (`Random`).
//! `seed = seed * 0x08088405 + 1 (mod 2^32)`; `Random(range) = (seed * range) >> 32`.

#[derive(Clone, Debug)]
pub struct DelphiRng {
    seed: u32,
}

impl DelphiRng {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    fn advance(&mut self) {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
    }

    /// `Random` without argument (`10c0:2167`): a real in `[0, 1)`: `(seed as i32 + 2^31) / 2^32` after advancing the seed.
    pub fn random_real(&mut self) -> f64 {
        self.advance();
        (self.seed as i32 as f64 + 2147483648.0) / 4294967296.0
    }

    /// `Random(range)`: uniform in `0..range`; `Random(0)` returns 0 like the original.
    pub fn random(&mut self, range: u16) -> u16 {
        self.advance();
        ((self.seed as u64 * range as u64) >> 32) as u16
    }
}

/// Literal translation of the 16-bit routine at `10c0:218a`, kept to prove the LCG form in tests.
#[cfg(test)]
fn advance_asm(seed: u32) -> u32 {
    let (mut ax, mut bx) = (seed as u16, (seed >> 16) as u16);
    let mut cx = ax;
    let prod = ax as u32 * 0x8405u32; // MUL word ptr CS:[21c0]
    ax = prod as u16;
    let mut dx = (prod >> 16) as u16;
    cx <<= 3; // three SHL CX,1
    let (mut ch, cl) = ((cx >> 8) as u8, cx as u8);
    ch = ch.wrapping_add(cl); // ADD CH,CL
    cx = ((ch as u16) << 8) | cl as u16;
    dx = dx.wrapping_add(cx).wrapping_add(bx);
    bx <<= 2;
    dx = dx.wrapping_add(bx);
    let mut dh = (dx >> 8) as u8;
    dh = dh.wrapping_add(bx as u8); // ADD DH,BL
    dx = ((dh as u16) << 8) | (dx & 0xff);
    bx <<= 5; // MOV CL,5 ; SHL BX,CL
    dh = (dx >> 8) as u8;
    dh = dh.wrapping_add(bx as u8);
    dx = ((dh as u16) << 8) | (dx & 0xff);
    let (sum, carry) = ax.overflowing_add(1);
    ax = sum;
    dx = dx.wrapping_add(carry as u16);
    ((dx as u32) << 16) | ax as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_matches_the_original_assembly() {
        let mut s = 1u32;
        for _ in 0..100_000 {
            let plain = s.wrapping_mul(0x0808_8405).wrapping_add(1);
            assert_eq!(advance_asm(s), plain, "seed {s:#x}");
            s = plain ^ s.rotate_left(7);
        }
    }

    #[test]
    fn random_real_is_in_the_unit_interval_and_matches_the_formula() {
        let mut r = DelphiRng::new(5);
        let mut probe = DelphiRng::new(5);
        let mut mean = 0.0;
        for _ in 0..20_000 {
            let x = r.random_real();
            assert!((0.0..1.0).contains(&x));
            mean += x;
            probe.random(1); // advance the seed once per draw
            assert_eq!(probe.seed(), r.seed());
        }
        assert!((mean / 20_000.0 - 0.5).abs() < 0.01);
    }

    #[test]
    fn random_is_in_range_and_roughly_uniform() {
        let mut r = DelphiRng::new(12345);
        let mut buckets = [0u32; 10];
        for _ in 0..100_000 {
            let v = r.random(10);
            assert!(v < 10);
            buckets[v as usize] += 1;
        }
        assert!(
            buckets.iter().all(|&b| (9_000..11_000).contains(&b)),
            "{buckets:?}"
        );
        assert_eq!(r.random(0), 0);
    }
}
