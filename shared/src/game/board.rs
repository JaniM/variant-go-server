use serde::{Deserialize, Serialize};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::Color;

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Board<T = Color> {
    pub width: u32,
    pub height: u32,
    pub points: Vec<T>,
}

pub type Point = (u32, u32);

impl<T: Copy + Default> Board<T> {
    pub fn empty(width: u32, height: u32) -> Self {
        Board {
            width,
            height,
            points: vec![T::default(); (width * height) as usize],
        }
    }

    pub fn point_within(&self, (x, y): Point) -> bool {
        (0..self.width).contains(&x) && (0..self.height).contains(&y)
    }

    pub fn get_point(&self, (x, y): Point) -> T {
        self.points[(y * self.width + x) as usize]
    }

    pub fn point_mut(&mut self, (x, y): Point) -> &mut T {
        &mut self.points[(y * self.width + x) as usize]
    }

    pub fn idx_to_coord(&self, idx: usize) -> Option<Point> {
        if idx < self.points.len() {
            Some((idx as u32 % self.width, idx as u32 / self.width))
        } else {
            None
        }
    }

    pub fn surrounding_points(&self, p: Point) -> impl Iterator<Item = Point> {
        let x = p.0 as i32;
        let y = p.1 as i32;
        let width = self.width;
        let height = self.height;
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .filter_map(move |&(dx, dy)| {
                if (x + dx) >= 0 && x + dx < width as i32 && (y + dy) >= 0 && y + dy < height as i32
                {
                    Some(((x + dx) as u32, (y + dy) as u32))
                } else {
                    None
                }
            })
    }
}

impl<T: Hash> Board<T> {
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(&self, &mut hasher);
        hasher.finish()
    }
}
