use rlbot::{
    Packet, RLBotConnection,
    flat::{self, DesiredBallState, DesiredCarState},
    util::RLBotEnvironment,
};

struct ZeroGScript {
    last_game_time: f32,
}

impl rlbot::scripts::Script for ZeroGScript {
    fn new(
        _agent_id: String,
        _match_configuration: rlbot::flat::MatchConfiguration,
        _field_info: rlbot::flat::FieldInfo,
        _packet_queue: &mut rlbot::util::PacketQueue,
    ) -> Self {
        Self {
            last_game_time: 0.0,
        }
    }

    fn tick(
        &mut self,
        game_packet: rlbot::flat::GamePacket,
        packet_queue: &mut rlbot::util::PacketQueue,
    ) {
        let delta_time = game_packet.match_info.seconds_elapsed - self.last_game_time;
        self.last_game_time = game_packet.match_info.seconds_elapsed;
        if game_packet.balls.len() == 0 {
            return;
        }
        packet_queue.push(Packet::DesiredGameState(flat::DesiredGameState {
            ball_states: game_packet
                .balls
                .iter()
                .map(|ball| {
                    let mut physics = ball.physics;
                    physics.velocity.z += 650.0 * delta_time;
                    DesiredBallState {
                        physics: Box::new(physics.into()),
                    }
                })
                .collect(),
            car_states: game_packet
                .players
                .iter()
                .map(|player| {
                    let mut physics = player.physics;
                    physics.velocity.z += 650.0 * delta_time;
                    DesiredCarState {
                        physics: Some(Box::new(physics.into())),
                        boost_amount: None,
                    }
                })
                .collect(),
            match_info: None,
            console_commands: vec![],
        }));
    }
}

fn main() {
    let RLBotEnvironment {
        server_addr,
        agent_id,
    } = RLBotEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "swz/zero-g-script".into());
    let rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    // Blocking.
    rlbot::scripts::run_script::<ZeroGScript>(agent_id.clone(), true, true, rlbot_connection)
        .expect("run_script crashed");

    println!("Script with agent_id `{agent_id}` exited nicely");
}
