use rlbot::{
    Packet, RLBotConnection,
    flat::{ConnectionSettings, ConsoleCommand, DesiredGameState, MatchPhase},
    util::RLBotEnvironment,
};

const TARGET_GRAVITY: f32 = -f32::MIN_POSITIVE;
// const TARGET_GRAVITY: f32 = -2000.0;

const COMMAND_REPEAT_INTERVAL: f32 = 0.05; // seconds
const COMMAND_REPEAT_DURATION: f32 = 0.5; // seconds

fn main() {
    let RLBotEnvironment {
        server_addr,
        agent_id,
    } = RLBotEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "swz/zero-g-script".into());
    let mut rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    rlbot_connection
        .send_packet(ConnectionSettings {
            agent_id: agent_id.clone(),
            close_between_matches: true,
            ..Default::default()
        })
        .expect("failed to send ConnectionSettings");

    rlbot_connection
        .send_packet(Packet::InitComplete)
        .expect("failed to send InitComplete");

    let mut prev_match_phase = MatchPhase::Inactive;
    let mut last_kickoff_time = 0.0;
    let mut last_application_time = 0.0;
    while let Ok(packet) = rlbot_connection.recv_packet() {
        if let Packet::GamePacket(game) = &packet {
            if game.match_info.match_phase == MatchPhase::Kickoff
                && prev_match_phase == MatchPhase::Countdown
            {
                last_kickoff_time = game.match_info.seconds_elapsed;
                println!("ZERO-G: Applying zero-g");
            }
            prev_match_phase = game.match_info.match_phase;

            // Repeat zero-g command for x seconds
            if game.match_info.seconds_elapsed - last_kickoff_time > COMMAND_REPEAT_DURATION {
                continue;
            }

            // Apply zero-g command every x seconds
            if game.match_info.seconds_elapsed - last_application_time >= COMMAND_REPEAT_INTERVAL {
                last_application_time = game.match_info.seconds_elapsed;
                rlbot_connection
                    .send_packet(DesiredGameState {
                        ball_states: vec![],
                        car_states: vec![],
                        match_info: None,
                        console_commands: vec![ConsoleCommand {
                            command: format!("Set WorldInfo WorldGravityZ {}", TARGET_GRAVITY),
                        }],
                    })
                    .expect("failed to send set gravity command");
            }
        }
        // idle until connection is closed
        if let Packet::None = packet {
            break;
        }
    }

    println!("Script with agent_id `{agent_id}` exited nicely");
}
