use rlbot::flat::{AirState, DesiredCarState, DesiredPhysics, Vector3, Vector3Partial};
use rlbot::{
    agents::run_script_agent,
    flat::{DesiredGameState, DesiredMatchInfo, MatchPhase},
    util::AgentEnvironment,
    RLBotConnection,
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
        }
    }

    fn tick(
        &mut self,
        game_packet: rlbot::flat::GamePacket,
        packet_queue: &mut rlbot::util::PacketQueue,
    ) {
        let dt = 1.0/120.0f32;
        let mut any_sticky = false;
        let mut car_states = vec![];
        for car in game_packet.players.iter() {
            let mut dcs = DesiredCarState::default();
            if car.air_state == AirState::OnGround || game_packet.match_info.match_phase == MatchPhase::Countdown {
                // Compute relative up and add extra sticky force
                let v = car.physics.velocity;
                let euler = car.physics.rotation;
                let cp = euler.pitch.cos();
                let sp = euler.pitch.sin();
                let cy = euler.yaw.cos();
                let sy = euler.yaw.sin();
                let cr = euler.roll.cos();
                let sr = euler.roll.sin();
                let up_x = -cr * cy * sp - sr * sy;
                let up_y = -cr * sy * sp + sr * cy;
                let up_z = cp * cr;
                dcs.physics = Some(DesiredPhysics {
                    location: None,
                    rotation: None,
                    velocity: Some(Vector3Partial::from(Vector3 {
                        x: v.x - up_x * EXTRA_STICKY_FORCE * dt,
                        y: v.y - up_y * EXTRA_STICKY_FORCE * dt,
                        z: v.z - up_z * EXTRA_STICKY_FORCE * dt,
                    }).into()),
                    angular_velocity: None,
                }.into());
                any_sticky = true;
            }
            car_states.push(dcs);
        }
        if any_sticky {
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
