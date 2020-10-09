use serde::{Deserialize, Serialize};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::Color;

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Board<T = Color> {
    pub width: u32,
    pub height: u32,
    pub toroidal: bool,
    pub points: Vec<T>,
}

pub type Point = (u32, u32);

impl<T: Copy + Default> Board<T> {
    pub fn empty(width: u32, height: u32, toroidal: bool) -> Self {
        Board {
            width,
            height,
            toroidal,
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

    pub fn wrap_point(&self, x: i32, y: i32) -> Option<Point> {
        wrap_point(x, y, self.width as i32, self.height as i32, self.toroidal)
    }

    pub fn surrounding_points(&self, p: Point) -> impl Iterator<Item = Point> {
        let x = p.0 as i32;
        let y = p.1 as i32;
        let width = self.width as i32;
        let height = self.height as i32;
        let toroidal = self.toroidal;
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .filter_map(move |&(dx, dy)| wrap_point(x + dx, y + dy, width, height, toroidal))
    }

    pub fn surrounding_diagonal_points(&self, p: Point) -> impl Iterator<Item = Point> {
        let x = p.0 as i32;
        let y = p.1 as i32;
        let width = self.width as i32;
        let height = self.height as i32;
        let toroidal = self.toroidal;
        [(-1, -1), (1, -1), (1, 1), (-1, 1)]
            .iter()
            .filter_map(move |&(dx, dy)| wrap_point(x + dx, y + dy, width, height, toroidal))
    }
}

impl<T: Hash> Board<T> {
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(&self, &mut hasher);
        hasher.finish()
    }
}

fn wrap_point(x: i32, y: i32, width: i32, height: i32, toroidal: bool) -> Option<Point> {
    if x >= 0 && x < width && y >= 0 && y < height {
        Some((x as u32, y as u32))
    } else if toroidal {
        let x = if x < 0 {
            x + width
        } else if x >= width {
            x - width
        } else {
            x
        };
        let y = if y < 0 {
            y + height
        } else if y >= height {
            y - height
        } else {
            y
        };
        Some((x as u32, y as u32))
    } else {
        None
    }
}
