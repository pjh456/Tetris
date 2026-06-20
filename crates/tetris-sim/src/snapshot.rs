use tetris_core::engine::Engine;
use tetris_protocol::newtypes::{Seed, TickNumber};
use tetris_protocol::protocol::{PacketHeader, PacketType, PktStateSnapshot};

pub fn build_snapshot(engine: &Engine<10, 20>, tick: TickNumber, seed: Seed) -> PktStateSnapshot {
    PktStateSnapshot {
        header: PacketHeader::new(PacketType::StateSnapshot, 0),
        tick,
        board_rows: engine.state.board.rows.to_vec(),
        piece: engine.state.piece,
        rot: engine.state.rot,
        x: engine.state.x,
        y: engine.state.y,
        hold: engine.state.hold,
        hold_used: engine.state.hold_used,
        next: engine.state.next,
        rng_state: engine.state.rng,
        combo: engine.state.combo,
        b2b: engine.state.b2b,
        pending_garbage: engine.state.pending_garbage,
        seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_through_bincode() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let snapshot = build_snapshot(&engine, TickNumber(7), Seed(42));
        let bytes = bincode::serialize(&snapshot).unwrap();
        let decoded: PktStateSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(decoded.tick, TickNumber(7));
    }
}
