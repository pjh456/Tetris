use std::time::Instant;
use tetris_core::engine::Engine;

fn main() {
    let mut engines = Vec::new();
    for i in 0..8 {
        let mut engine = Engine::<10, 20>::new();
        engine.reset((42 + i * 100) as u32);
        engines.push(engine);
    }

    for _ in 0..100 {
        tick_all(&mut engines);
    }

    let iterations = 1000u32;
    let start = Instant::now();
    for _ in 0..iterations {
        tick_all(&mut engines);
    }
    let elapsed = start.elapsed();

    println!("8 players x {} ticks: {:?}", iterations, elapsed);
    println!("Average per tick: {:?}", elapsed / iterations);
    println!(
        "Average per player per tick: {:?}",
        elapsed / (iterations * 8)
    );

    if elapsed.as_millis() < 5000 {
        println!("PASS: under 5 seconds");
    } else {
        println!("FAIL: over 5 seconds (target: < 5s)");
    }
}

fn tick_all(engines: &mut [Engine<10, 20>]) {
    for engine in engines {
        engine.tick(16);
    }
}
