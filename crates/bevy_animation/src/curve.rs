use std::any::TypeId;

use crate::lerping::Lerp;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// https://github.com/niklasfrykholm/blog
// https://bitsquid.blogspot.com/search?q=animation
// http://bitsquid.blogspot.com/2009/11/bitsquid-low-level-animation-system.html
// http://archive.gamedev.net/archive/reference/articles/article1497.html (bit old)

// http://nfrechette.github.io/2016/12/07/anim_compression_key_reduction/
// https://github.com/nfrechette/acl

// TODO: impl Serialize, Deserialize
#[derive(Default, Debug)]
pub struct Curve<T> {
    frame_rate: u32,
    offset: u16,
    values: Vec<T>,
}

impl<T: Clone> Clone for Curve<T> {
    fn clone(&self) -> Self {
        Self {
            frame_rate: self.frame_rate,
            offset: self.offset,
            values: self.values.clone(),
        }
    }
}

impl<T> Curve<T> {
    pub fn from_samples(samples: Vec<f32>, values: Vec<T>) -> Self {
        // Guesses the frame rate or defaults to 30
        let frame_rate = if samples.len() < 2 {
            30
        } else {
            let r = 1.0 / (samples[1] - samples[0]);
            r as u32
        };

        Self::from_samples_and_rate(frame_rate, samples, values)
    }

    pub fn from_samples_and_rate(frame_rate: u32, samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        // Make sure both have the same length
        assert!(
            samples.len() == values.len(),
            "samples and values must have the same length"
        );

        assert!(
            values.len() <= u16::MAX as usize,
            "limit of {} keyframes exceeded",
            u16::MAX
        );

        assert!(samples.len() > 0, "empty curve");

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );

        let curve = Self {
            frame_rate,
            offset: (samples[0] * frame_rate as f32) as u16,
            values,
        };

        curve
    }

    pub fn from_line(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        todo!()
        // let frame_duration = 1.0 / (t1 - t0);
        // Self {
        //     frame_rate: 30,
        //     offset: (t0 * frame_duration) as u16,
        //     values: vec![v0, v1],
        // }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            frame_rate: 30,
            offset: 0,
            values: vec![v],
        }
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    //assert!(samples.len() > 1, "curve can't be empty");
    // }

    pub const fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    pub fn duration(&self) -> f32 {
        debug_assert!(self.values.len() > 0, "curve has no keyframes");
        (self.values.len() - 1 + self.offset as usize) as f32 / self.frame_rate as f32
    }

    pub fn trim(&mut self, keyframes: u16) {
        self.offset -= keyframes.max(self.offset);
    }

    // pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
    //     self.samples.iter().copied().zip(self.values.iter())
    // }

    // pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
    //     self.samples.iter().copied().zip(self.values.iter_mut())
    // }
}

impl<T> Curve<T>
where
    T: Lerp + Clone + 'static,
{
    // TODO: Profile sample_indexed vs sample, I want to know when use one over the other?

    /// Easer to use sampling method that don't have time restrictions or needs
    /// the keyframe index, but is more expensive always `O(n)`. Which means
    /// sampling takes longer to evaluate as much as time get closer to curve duration
    /// and it get worse with more keyframes.
    pub fn sample(&self, time: f32) -> T {
        // Don't care about the time
        self.sample_indexed(0, time).1
    }

    /// Samples the curve starting from some keyframe index, this make the common case `O(1)`
    ///
    /// **NOTE** Each keyframe is indexed by a `u16` to reduce memory usage when using the keyframe caching
    pub fn sample_indexed(&self, index: u16, time: f32) -> (u16, T) {
        // Adjust for the current keyframe index
        let t = time * self.frame_rate as f32 - self.offset as f32;
        if t.is_sign_negative() {
            println!("under {:?}", t);
            return (0, self.values[0].clone());
        }

        let f = t.trunc() as usize;

        let f_n = self.values.len() - 1;
        if f >= f_n {
            println!("over {:?}", (f, t));
            return (f_n as u16, self.values[f_n].clone());
        }

        let t = t.fract();
        // Lerp the value
        let value = T::lerp(&self.values[f], &self.values[f + 1], t);

        (0, value)
    }

    #[inline(always)]
    pub fn value_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[inline(always)]
    pub fn value_size(&self) -> usize {
        std::mem::size_of::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_evaluation() {
        let curve = Curve::from_samples(
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
        );
        assert_eq!(curve.sample(0.5), 1.0);

        let mut i0 = 0;
        let mut e0 = 0.0;
        for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
            let v = *v;
            let (i1, e1) = curve.sample_indexed(i0, v);
            assert_eq!(e1, 2.0 * v);
            if e1 > e0 {
                assert!(i1 >= i0);
            } else {
                assert!(i1 <= i0);
            }
            e0 = e1;
            i0 = i1;
        }
    }

    #[test]
    #[should_panic]
    fn curve_bad_length() {
        let _ = Curve::from_samples(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = Curve::from_samples(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
    }
}
