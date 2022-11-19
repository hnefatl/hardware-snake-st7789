use core::{convert::Infallible, iter::Cycle, slice::Iter};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, RgbColor, Size},
    primitives::Rectangle,
};
use hash32::{Hash, Hasher};
use heapless::{self, FnvIndexMap};
use st7789::Error;

use crate::inputs::{Direction, GameInputs};

/// A position on the screen. (0, 0) is the top-left of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: u8,
    y: u8,
}
impl Hash for Point {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.x.hash(state);
        self.y.hash(state);
    }
}
impl Point {
    pub fn new(x: u8, y: u8) -> Self {
        Point { x, y }
    }
}
// TODO: consider just using the primitive point.
impl Into<embedded_graphics::prelude::Point> for Point {
    fn into(self) -> embedded_graphics::prelude::Point {
        embedded_graphics::prelude::Point::new(self.x as i32, self.y as i32)
    }
}
/// A delta for a Point.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
struct Vector {
    dx: i8,
    dy: i8,
}
impl Vector {
    pub fn new(dx: i8, dy: i8) -> Self {
        Vector { dx, dy }
    }

    pub fn opposite(&self) -> Self {
        Vector { dx: self.dx * -1, dy: self.dy * -1 }
    }
}
impl Into<Vector> for Direction {
    fn into(self) -> Vector {
        match self {
            Direction::Up => Vector::new(0, -1),
            Direction::Down => Vector::new(0, 1),
            Direction::Left => Vector::new(-1, 0),
            Direction::Right => Vector::new(1, 0),
        }
    }
}

/// Render a logical game position as pixels on the screen, with upscaling.
fn _render_point<const PIXEL_WIDTH: u8, R>(point: &Point, colour: Rgb565, target: &mut R)
where
    R: DrawTarget<Color = Rgb565, Error = Error<Infallible>>,
{
    let size = Size {
        width: PIXEL_WIDTH as u32,
        height: PIXEL_WIDTH as u32,
    };
    let top_left = embedded_graphics::prelude::Point {
        x: point.x as i32 * PIXEL_WIDTH as i32,
        y: point.y as i32 * PIXEL_WIDTH as i32,
    };
    target.fill_solid(&Rectangle { top_left, size }, colour).unwrap();
}

struct Snake<const GAME_WIDTH: u8, const GAME_HEIGHT: u8, const PIXEL_WIDTH: u8>
where
    [(); GAME_WIDTH as usize * GAME_HEIGHT as usize]:,
{
    /// The vertices the snake currently occupies. The `front()` of the deque is the snake's head.
    points: heapless::Deque<Point, { GAME_WIDTH as usize * GAME_HEIGHT as usize }>,
    /// The last occupied tail position, used for wiping the background.
    old_tail: Option<Point>,
    direction: Direction,
}
impl<const GAME_WIDTH: u8, const GAME_HEIGHT: u8, const PIXEL_WIDTH: u8> Snake<GAME_WIDTH, GAME_HEIGHT, PIXEL_WIDTH>
where
    [(); GAME_WIDTH as usize * GAME_HEIGHT as usize]:,
{
    const COLOUR: Rgb565 = Rgb565::GREEN;

    pub fn new(initial_point: Point, initial_direction: Direction) -> Self {
        let mut points = heapless::Deque::new();
        points.push_back(initial_point).unwrap();
        Snake {
            points,
            direction: initial_direction,
            old_tail: None,
        }
    }

    /// Move the snake in the current direction.
    pub fn update<const N: usize>(&mut self, food: &mut FnvIndexMap<Point, Food, N>) {
        let Some(old_head) = self.points.front() else {
            return
        };
        let direction_delta: Vector = self.direction.into();
        let new_head = Self::_add_with_wraparound(old_head.clone(), direction_delta);

        let ate_food = food.remove(&new_head).is_some();
        if !ate_food {
            // If we didn't eat a food, remove the last tail location to make up for the head moving. If we did eat food,
            // leave the tail point where it is so that we increase our length by 1.
            self.old_tail = self.points.pop_back();
        }

        self.points.push_front(new_head).unwrap();
    }

    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction
    }
    pub fn get_direction(&self) -> Direction {
        self.direction
    }

    fn render<R>(&self, target: &mut R)
    where
        R: DrawTarget<Color = Rgb565, Error = Error<Infallible>>,
    {
        // Blank out the old tail position.
        if let Some(old_point) = self.old_tail {
            _render_point::<PIXEL_WIDTH, R>(&old_point, Rgb565::BLACK, target);
        }
        for point in self.points.iter() {
            _render_point::<PIXEL_WIDTH, R>(point, Self::COLOUR, target);
        }
    }

    pub fn length(&self) -> usize {
        self.points.len()
    }

    pub fn get_head(&self) -> Point {
        self.points.front().unwrap().clone()
    }

    fn _add_with_wraparound(point: Point, delta: Vector) -> Point {
        let x = point.x as i16 + delta.dx as i16;
        let y = point.y as i16 + delta.dy as i16;
        // Euclidean remainder handles wraparound nicely, taking negative values to e.g. width - abs(value).
        Point {
            x: x.rem_euclid(GAME_WIDTH as i16) as u8,
            y: y.rem_euclid(GAME_HEIGHT as i16) as u8,
        }
    }
}

#[derive(Debug)]
struct Food {
    colours: Cycle<Iter<'static, Rgb565>>,
    next_colour: Rgb565,
}
impl Food {
    const COLOURS: &'static [Rgb565] = &[Rgb565::WHITE, Rgb565::RED];

    fn new() -> Self {
        let mut colours = Self::COLOURS.iter().cycle();
        let next_colour = *colours.next().unwrap();
        Food { colours, next_colour }
    }

    fn update(&mut self) {
        self.next_colour = *self.colours.next().unwrap();
    }

    fn render<const GAME_WIDTH: u8, const GAME_HEIGHT: u8, const PIXEL_WIDTH: u8, R>(
        &self,
        point: &Point,
        target: &mut R,
    ) where
        R: DrawTarget<Color = Rgb565, Error = Error<Infallible>>,
    {
        _render_point::<PIXEL_WIDTH, R>(point, self.next_colour, target)
    }
}

pub struct Game<const GAME_WIDTH: u8, const GAME_HEIGHT: u8, const PIXEL_WIDTH: u8>
where
    [(); GAME_WIDTH as usize * GAME_HEIGHT as usize]:,
{
    snake: Snake<GAME_WIDTH, GAME_HEIGHT, PIXEL_WIDTH>,
    inputs: GameInputs,
    food: heapless::FnvIndexMap<Point, Food, 8>,
    /// The number of food items to maintain on the board.
    num_food: usize,
}
impl<const GAME_WIDTH: u8, const GAME_HEIGHT: u8, const PIXEL_WIDTH: u8> Game<GAME_WIDTH, GAME_HEIGHT, PIXEL_WIDTH>
where
    [(); GAME_WIDTH as usize * GAME_HEIGHT as usize]:,
{
    pub fn new(inputs: GameInputs) -> Self {
        Game {
            snake: Snake::new(Point::new(GAME_WIDTH / 2, GAME_HEIGHT / 2), Direction::Right),
            inputs,
            food: heapless::FnvIndexMap::new(),
            num_food: 1,
        }
    }

    /// The "fast" update cycle, for input/non-snake-"step" updates.
    pub fn fast_update(&mut self) {
        if let Some(direction) = self.inputs.get_joystick_direction() {
            // Only change direction if it's not the opposite direction to the current.
            if Into::<Vector>::into(direction).opposite() != self.snake.get_direction().into() {
                self.snake.set_direction(direction);
            }
        }
    }

    /// The "slow" update cycle, once every game "step" (snake movement).
    pub fn slow_update(&mut self) {
        self.snake.update(&mut self.food);

        while self.food.len() < self.num_food {
            if self.snake.get_head().y < 5 && self.snake.length() < 5 {
                break;
            }
            self.food.insert(Point::new(12, 5), Food::new()).unwrap();
        }

        for food in self.food.values_mut() {
            food.update();
        }
    }

    pub fn render<R>(&self, target: &mut R)
    where
        R: DrawTarget<Color = Rgb565, Error = Error<Infallible>>,
    {
        for (point, food) in self.food.iter() {
            food.render::<GAME_WIDTH, GAME_HEIGHT, PIXEL_WIDTH, R>(point, target);
        }
        self.snake.render(target);
    }
}
