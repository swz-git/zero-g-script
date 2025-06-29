use rlbot::flat::{AirState, DesiredCarState, Vector3, Vector3Partial};
use rlbot::{
    RLBotConnection,
    agents::run_script_agent,
    flat::{DesiredGameState, DesiredMatchInfo, MatchPhase},
    util::AgentEnvironment,
};

const TARGET_GRAVITY: f32 = -f32::MIN_POSITIVE;
// const TARGET_GRAVITY: f32 = -2000.0;

const EXTRA_STICKY_FORCE: f32 = 300.0;

const COMMAND_REPEAT_INTERVAL: f32 = 0.05; // seconds
const COMMAND_REPEAT_DURATION: f32 = 0.5; // seconds

struct ZeroGScript {
    prev_match_phase: MatchPhase,
    last_kickoff_time: f32,
    last_application_time: f32,
    prev_seconds_elapsed: f32,
}

impl rlbot::agents::ScriptAgent for ZeroGScript {
    fn new(
        _agent_id: String,
        _match_configuration: rlbot::flat::MatchConfiguration,
        _field_info: rlbot::flat::FieldInfo,
        _packet_queue: &mut rlbot::util::PacketQueue,
    ) -> Self {
        Self {
            prev_match_phase: MatchPhase::Inactive,
            last_kickoff_time: 0.0,
            last_application_time: 0.0,
            prev_seconds_elapsed: 0.0,
        }
    }

    fn tick(
        &mut self,
        game_packet: rlbot::flat::GamePacket,
        packet_queue: &mut rlbot::util::PacketQueue,
    ) {
        let dt = game_packet.match_info.seconds_elapsed - self.prev_seconds_elapsed;
        self.prev_seconds_elapsed = game_packet.match_info.seconds_elapsed;

        let mut car_states = vec![DesiredCarState::default(); game_packet.players.len()];
        car_states
            .iter_mut()
            .zip(game_packet.players.iter())
            .filter(|(_, car)| {
                car.air_state == AirState::OnGround
                    || game_packet.match_info.match_phase == MatchPhase::Countdown
            })
            .for_each(|(dcs, car)| {
                // Compute relative up and add extra sticky force
                let r = car.physics.rotation; // Euler rot
                let up_x = -r.roll.cos() * r.yaw.cos() * r.pitch.sin() - r.roll.sin() * r.yaw.sin();
                let up_y = -r.roll.cos() * r.yaw.sin() * r.pitch.sin() + r.roll.sin() * r.yaw.cos();
                let up_z = r.pitch.cos() * r.roll.cos();

                let physics = dcs.physics.get_or_insert_default();
                physics.velocity = Some(
                    Vector3Partial::from(Vector3 {
                        x: car.physics.velocity.x - up_x * EXTRA_STICKY_FORCE * dt,
                        y: car.physics.velocity.y - up_y * EXTRA_STICKY_FORCE * dt,
                        z: car.physics.velocity.z - up_z * EXTRA_STICKY_FORCE * dt,
                    })
                    .into(),
                )
            });

        if car_states.iter().any(|x| x.physics.is_some()) {
            packet_queue.push(DesiredGameState {
                ball_states: vec![],
                car_states,
                match_info: None,
                console_commands: vec![],
            });
        }

        if game_packet.match_info.match_phase == MatchPhase::Kickoff
            && self.prev_match_phase == MatchPhase::Countdown
        {
            self.last_kickoff_time = game_packet.match_info.seconds_elapsed;
            println!("ZERO-G: Applying zero-g");
        }
        self.prev_match_phase = game_packet.match_info.match_phase;

        // Repeat zero-g command for x seconds
        if game_packet.match_info.seconds_elapsed - self.last_kickoff_time > COMMAND_REPEAT_DURATION
        {
            return;
        }

        // Apply zero-g command every x seconds
        if game_packet.match_info.seconds_elapsed - self.last_application_time
            >= COMMAND_REPEAT_INTERVAL
        {
            self.last_application_time = game_packet.match_info.seconds_elapsed;
            packet_queue.push(DesiredGameState {
                ball_states: vec![],
                car_states: vec![],
                match_info: Some(Box::new(DesiredMatchInfo {
                    world_gravity_z: Some(TARGET_GRAVITY.into()),
                    game_speed: None,
                })),
                console_commands: vec![],
            });
        }
    }
}

fn main() {
    let AgentEnvironment {
        server_addr,
        agent_id,
    } = AgentEnvironment::from_env();
    let agent_id = agent_id.unwrap_or_else(|| "swz/zero-g-script".into());
    let rlbot_connection = RLBotConnection::new(&server_addr).expect("connection");

    run_script_agent::<ZeroGScript>(agent_id.clone(), false, false, rlbot_connection)
        .expect("run_script_agent failed");

    println!("Script with agent_id `{agent_id}` exited nicely");
}
