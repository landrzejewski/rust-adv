use std::fmt;

// ============================
// State markers
// ============================

struct Red;
struct Green;
struct Yellow;
struct FlashingRed {
    reason: String,
}

// ============================
// TrafficLight<S>
// ============================

struct TrafficLight<S> {
    name: String,
    state: S,
}

// ============================
// Normal transitions (branching graph)
// Red -> Green, Green -> Yellow, Yellow -> Red
// ============================

impl TrafficLight<Red> {
    fn new(name: &str) -> Self {
        TrafficLight {
            name: name.to_string(),

            state: Red,
        }
    }

    fn turn_green(self) -> TrafficLight<Green> {
        TrafficLight {
            name: self.name,

            state: Green,
        }
    }
}

impl TrafficLight<Green> {
    fn turn_yellow(self) -> TrafficLight<Yellow> {
        TrafficLight {
            name: self.name,

            state: Yellow,
        }
    }
}

impl TrafficLight<Yellow> {
    fn turn_red(self) -> TrafficLight<Red> {
        TrafficLight {
            name: self.name,

            state: Red,
        }
    }
}

impl TrafficLight<FlashingRed> {
    fn recover(self) -> TrafficLight<Red> {
        TrafficLight {
            name: self.name,

            state: Red,
        }
    }
}

// ============================
// Emergency trait: any state -> FlashingRed
// ============================

trait Emergency {
    fn emergency(self, reason: &str) -> TrafficLight<FlashingRed>;
}

impl<S> Emergency for TrafficLight<S> {
    fn emergency(self, reason: &str) -> TrafficLight<FlashingRed> {
        TrafficLight {
            name: self.name,

            state: FlashingRed {
                reason: reason.to_string(),
            },
        }
    }
}

// ============================
// LightStatus trait — enables heterogeneous Vec<Box<dyn LightStatus>>
// ============================

trait LightStatus: fmt::Display {
    fn status_line(&self) -> String;
    fn is_stop(&self) -> bool;
}

impl LightStatus for TrafficLight<Red> {
    fn status_line(&self) -> String {
        "RED — Stop".to_string()
    }
    fn is_stop(&self) -> bool {
        true
    }
}

impl LightStatus for TrafficLight<Green> {
    fn status_line(&self) -> String {
        "GREEN — Go".to_string()
    }
    fn is_stop(&self) -> bool {
        false
    }
}

impl LightStatus for TrafficLight<Yellow> {
    fn status_line(&self) -> String {
        "YELLOW — Caution".to_string()
    }
    fn is_stop(&self) -> bool {
        true // should stop if possible
    }
}

impl LightStatus for TrafficLight<FlashingRed> {
    fn status_line(&self) -> String {
        format!("FLASHING RED — Emergency: {}", self.state.reason)
    }
    fn is_stop(&self) -> bool {
        true
    }
}

// Display for all TrafficLight variants (delegates to status_line)
impl<S> fmt::Display for TrafficLight<S>
where
    TrafficLight<S>: LightStatus,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.name, self.status_line())
    }
}

// ============================
// Intersection: managing multiple lights
// ============================

struct Intersection {
    lights: Vec<Box<dyn LightStatus>>,
}

impl Intersection {
    fn new() -> Self {
        Intersection { lights: Vec::new() }
    }

    fn add(&mut self, light: Box<dyn LightStatus>) {
        self.lights.push(light);
    }

    fn all_stopped(&self) -> bool {
        self.lights.iter().all(|l| l.is_stop())
    }

    fn report(&self) {
        for light in &self.lights {
            println!("  {light}");
        }
    }
}

// ============================
// Demonstration
// ============================

pub fn run() {
    println!("=== Exercise 10: Traffic Light Controller ===\n");

    // --- Normal transitions ---
    println!("--- Normal transition cycle ---");
    let light = TrafficLight::<Red>::new("Main St North");
    println!("  {light}");

    let light = light.turn_green();
    println!("  {light}");

    let light = light.turn_yellow();
    println!("  {light}");

    let light = light.turn_red();
    println!("  {light}");

    // --- Emergency from any state ---
    println!("\n--- Emergency transitions ---");
    let green_light = TrafficLight::<Red>::new("Elm St").turn_green();
    println!("  Before: {green_light}");
    let flashing = green_light.emergency("Power grid failure");
    println!("  After:  {flashing}");

    // Emergency from Yellow
    let yellow_light = TrafficLight::<Red>::new("Oak Ave").turn_green().turn_yellow();
    let flashing2 = yellow_light.emergency("Accident ahead");
    println!("  From yellow: {flashing2}");

    // Recover from flashing
    let recovered = flashing.recover();
    println!("  Recovered:   {recovered}");

    // --- Intersection with heterogeneous lights ---
    println!("\n--- Intersection management ---");
    let mut intersection = Intersection::new();

    // Create 4 lights in different states
    let north = TrafficLight::<Red>::new("North");
    let south = TrafficLight::<Red>::new("South").turn_green();
    let east = TrafficLight::<Red>::new("East").turn_green().turn_yellow();
    let west = TrafficLight::<Red>::new("West").emergency("Sensor fault");

    intersection.add(Box::new(north));
    intersection.add(Box::new(south));
    intersection.add(Box::new(east));
    intersection.add(Box::new(west));

    intersection.report();
    println!("  All stopped? {}", intersection.all_stopped());

    // --- All-red intersection ---
    println!("\n--- All-red intersection ---");
    let mut all_red = Intersection::new();
    all_red.add(Box::new(TrafficLight::<Red>::new("N")));
    all_red.add(Box::new(TrafficLight::<Red>::new("S")));
    all_red.add(Box::new(TrafficLight::<Red>::new("E")));
    all_red.add(Box::new(TrafficLight::<Red>::new("W")));
    all_red.report();
    println!("  All stopped? {}", all_red.all_stopped());

    // Invalid transitions are compile errors:
    // TrafficLight::<Red>::new("X").turn_yellow(); // Error: no turn_yellow on Red
    // TrafficLight::<Red>::new("X").turn_green().turn_red(); // Error: no turn_red on Green
}
