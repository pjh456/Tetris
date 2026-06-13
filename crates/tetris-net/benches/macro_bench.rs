use std::time::Instant;
use tetris_core::engine::Engine;
use tetris_net::net_game_driver::NetGameDriver;

fn main() {
    let mut driver = NetGameDriver::<10, 20>::new(Engine::new());
    for i in 0..7 {
        let mut engine = Engine::<10, 20>::new();
        engine.reset((42 + i * 100) as u32);
        driver.add_player(engine);
    }

    for _ in 0..100 {
        driver.tick_all(16);
    }

    let iterations = 1000u32;
    let start = Instant::now();
    for _ in 0..iterations {
        driver.tick_all(16);
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
