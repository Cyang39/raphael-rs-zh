pub struct NamedTimer {
    name: &'static str,
    #[cfg(not(target_arch = "wasm32"))]
    timer: std::time::Instant,
}

impl NamedTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            #[cfg(not(target_arch = "wasm32"))]
            timer: std::time::Instant::now(),
        }
    }
}

impl Drop for NamedTimer {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        eprintln!("{}: (timer not available on WASM)", self.name);
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!(
            "{}: {} seconds",
            self.name,
            self.timer.elapsed().as_secs_f32()
        );
    }
}

pub struct Backtracking<T: Copy> {
    entries: Vec<(T, u32)>,
}

impl<T: Copy> Backtracking<T> {
    pub const SENTINEL: u32 = u32::MAX;

    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, mut index: u32) -> impl Iterator<Item = T> {
        let mut items = Vec::new();
        while index != Self::SENTINEL {
            items.push(self.entries[index as usize].0);
            index = self.entries[index as usize].1;
        }
        items.into_iter().rev()
    }

    pub fn push(&mut self, item: T, parent: u32) -> u32 {
        self.entries.push((item, parent));
        self.entries.len() as u32 - 1
    }
}

impl<T: Copy> Drop for Backtracking<T> {
    fn drop(&mut self) {
        dbg!(self.entries.len());
    }
}