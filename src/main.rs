use sfml::{audio::*, graphics::*, system::*, window::*};
use std::f32::consts::PI;

const TARGET_FPS: u32 = 240;
const WIDTH: u32 = 1200;
const HEIGHT: u32 = 800;
const ARC_COUNT: u32 = 21;
const ARC_CENTER: (f32, f32) = (WIDTH as f32 * 0.5, HEIGHT as f32 * 0.9);
const TIME_SECS: f32 = 900.0; // in seconds
const GLOW_DURATION: f32 = 500.0; // in milliseconds

fn main() {
    let context_settings = ContextSettings {
        antialiasing_level: 8,
        ..Default::default()
    };

    let mut window = RenderWindow::new(
        (WIDTH, HEIGHT),
        "Polyrhythm",
        Style::CLOSE,
        &context_settings,
    );

    window.set_framerate_limit(TARGET_FPS);

    let mut info_text = {
        let mut font: Box<sfml::SfBox<Font>> = Box::new(Font::from_file("Hack NF.ttf").unwrap());
        font.set_smooth(true);
        let mut text = Text::default();
        text.set_string("");
        text.set_font(Box::leak(font));
        text.set_character_size(20);
        text
    };

    info_text.set_position((10.0, 10.0));
    info_text.set_fill_color(Color::WHITE);

    let mut polyrhythm = Polyrhythm::new();
    let mut dtc = Clock::start();

    while window.is_open() {
        let dt = dtc.restart();
        while let Some(event) = window.poll_event() {
            if let Event::Closed = event {
                window.close();
            }
        }

        let fps = 1.0 / dt.as_seconds();
        info_text.set_string(&format!(
            "FPS: {:.0}\nCollisions: {}\nTime Elapsed: {:.0}s/{TIME_SECS:.0}s",
            fps,
            polyrhythm.num_collisions,
            polyrhythm.elapsed_time.as_seconds(),
        ));

        window.clear(Color::BLACK);
        polyrhythm.draw(&mut window, dt); // draw arcs, circles and play audio
        window.draw(&info_text);
        window.display();
    }
}

struct Polyrhythm<'a> {
    /// Arcs
    arcs: Vec<Arc<'a>>,

    /// The rectangle covering parts of circle so that it looks like an arc
    rect: RectangleShape<'a>,

    /// The little circle on each arc
    circle: CircleShape<'a>,

    /// The elasped time since the program is running
    elapsed_time: Time,

    /// Player
    players: Vec<Music<'a>>,

    /// Number of times ball has touched either side
    num_collisions: usize,

    collision: Vec<bool>,
}

impl<'a> Polyrhythm<'a> {
    fn new() -> Self {
        Self {
            num_collisions: 0,
            players: (0..ARC_COUNT)
                .map(|i| {
                    let mut m = Music::from_file(&format!("sounds/key-{i}.wav")).unwrap();
                    m.set_volume(15.0);
                    m
                })
                .collect(),
            collision: (0..ARC_COUNT).map(|_| false).collect(),
            elapsed_time: Time::milliseconds(100),
            circle: {
                let radius = 5.0;
                let mut circle = CircleShape::new(radius, 50);
                circle.set_fill_color(Color::WHITE);
                circle.set_position((0.0, 0.0));
                circle.set_origin((radius, radius));
                circle
            },
            rect: {
                let mut rect = RectangleShape::new();
                rect.set_fill_color(Color::BLACK);
                rect.set_position((0.0, HEIGHT as f32 * 0.9));
                rect.set_size((WIDTH as f32, 300.0));
                rect
            },
            arcs: (0..ARC_COUNT)
                .map(|i| {
                    let radius =
                        50.0 + ((WIDTH as f32 / 2.0) / (ARC_COUNT as f32 + 3.0) * i as f32);
                    Arc::new(radius)
                })
                .collect::<Vec<_>>(),
        }
    }

    fn draw(&mut self, window: &mut RenderWindow, dt: Time) {
        self.elapsed_time += dt;

        for idx in 0..self.arcs.len() {
            let itm = self.arcs.get_mut(idx).unwrap();
            let collision = self.collision.get_mut(idx).unwrap();

            if *collision {
                itm.glow_start();

                // play sound
                if self.players[idx].status() != SoundStatus::PLAYING {
                    self.num_collisions += 1;
                    self.players[idx].play();
                }

                *collision = false;
            }

            itm.draw(window, dt);
        }

        window.draw(&self.rect);

        for i in 0..ARC_COUNT {
            let arc_radius = 50.0 + ((WIDTH as f32 / 2.0) / (ARC_COUNT as f32 + 3.0) * i as f32);

            static ONE_LOOP: f32 = 2.0 * PI;
            let speed = (ONE_LOOP * (50 - i) as f32) / TIME_SECS;
            let distance = PI + speed * self.elapsed_time.as_seconds();
            let mod_distance = distance % (2.0 * PI);
            let adjusted_distance = if mod_distance >= PI {
                mod_distance
            } else {
                2.0 * PI - mod_distance
            };

            let (x, y) = (
                ARC_CENTER.0 + arc_radius * adjusted_distance.cos(),
                ARC_CENTER.1 + arc_radius * adjusted_distance.sin(),
            );

            if (0.0..0.005).contains(&(distance % PI)) {
                self.collision[i as usize] = true;
            }

            self.circle.set_position((x, y));

            window.draw(&self.circle);
        }
    }
}

struct Arc<'a> {
    glow_start_time: Option<Time>,
    arc_shape: CircleShape<'a>,
    elapsed_time: Time,
}

impl<'a> Arc<'a> {
    fn new(radius: f32) -> Self {
        Self {
            glow_start_time: None,
            arc_shape: {
                let mut arc = CircleShape::new(radius, 100);
                arc.set_origin((radius, radius));
                arc.set_outline_thickness(2.0);
                arc.set_outline_color(Color::rgb(50, 50, 50));
                arc.set_position(ARC_CENTER);
                arc.set_fill_color(Color::TRANSPARENT);

                arc
            },
            elapsed_time: Time::ZERO,
        }
    }

    fn draw(&mut self, window: &mut RenderWindow, dt: Time) {
        self.elapsed_time += dt;

        if let Some(glow_start_time) = self.glow_start_time {
            let time_since_glow_start = self.elapsed_time - glow_start_time;

            if time_since_glow_start.as_milliseconds() > GLOW_DURATION as i32 {
                // fade out

                let fade_factor = (time_since_glow_start.as_milliseconds() as f32 - GLOW_DURATION)
                    / GLOW_DURATION;
                let fade_factor = 1.0 - fade_factor.clamp(0.196_078_43, 1.0);

                let color_val = ((255.0 * fade_factor) as u8).clamp(50, 255);
                self.arc_shape
                    .set_outline_color(Color::rgb(color_val, color_val, color_val));
            } else {
                // fade in

                let fade_factor = time_since_glow_start.as_milliseconds() as f32 / GLOW_DURATION;
                let fade_factor = fade_factor.clamp(0.196_078_43, 1.0);
                let color_val = (255.0 * fade_factor) as u8;
                self.arc_shape
                    .set_outline_color(Color::rgb(color_val, color_val, color_val));
            }
        }

        window.draw(&self.arc_shape);
    }

    fn glow_start(&mut self) {
        self.glow_start_time = Some(self.elapsed_time);
    }
}
