use std::rc::Rc;

use super::*;
use crate::nodes;
use crate::scene_fields::SceneValue;
use crate::test_host::{TestHost, TestNode};
use crate::tracks::{self, Ease, Track, TrackClip, TrackKey, Tracks};

const EPSILON: f32 = 1e-4;

/// An animation of one second that slides the parent's `x` from `from`
/// to `to`.
fn slide(name: &str, from: f64, to: f64, looping: bool) -> TrackClip {
    TrackClip {
        name: name.to_owned(),
        length: 1.0,
        looping,
        tracks: vec![Track {
            node: String::new(),
            property: "x".to_owned(),
            ease: Ease::Linear,
            keys: vec![
                TrackKey {
                    time: 0.0,
                    value: SceneValue::Float(from),
                },
                TrackKey {
                    time: 1.0,
                    value: SceneValue::Float(to),
                },
            ],
        }],
        calls: Vec::new(),
    }
}

fn animations() -> Tracks {
    Tracks {
        animations: vec![
            slide("idle", 0.0, 0.0, true),
            slide("walk", 10.0, 10.0, true),
            slide("run", 20.0, 20.0, true),
            slide("jump", 0.0, 100.0, false),
        ],
    }
}

fn condition(value: &str, test: Test) -> Condition {
    Condition {
        value: value.to_owned(),
        test,
    }
}

fn transition(from: Option<&str>, to: &str, conditions: Vec<Condition>) -> Transition {
    Transition {
        from: from.map(str::to_owned),
        to: to.to_owned(),
        conditions,
        fade: 0.0,
        at_end: false,
    }
}

fn value(name: &str, kind: ValueKind) -> MachineValue {
    MachineValue {
        name: name.to_owned(),
        kind,
    }
}

fn hero_machine() -> StateMachine {
    let state = |name: &str, motion: Motion| MachineState {
        name: name.to_owned(),
        motion,
    };
    let point = |animation: &str, at: f32| BlendPoint {
        animation: animation.to_owned(),
        at,
    };
    let mut to_move = transition(
        Some("idle"),
        "move",
        vec![condition("speed", Test::Above(0.1))],
    );
    to_move.fade = 0.5;
    let mut landed = transition(Some("jump"), "idle", Vec::new());
    landed.at_end = true;
    StateMachine {
        values: vec![
            value("speed", ValueKind::Number),
            value("grounded", ValueKind::Flag),
            value("jump", ValueKind::Trigger),
        ],
        start: "idle".to_owned(),
        states: vec![
            state("idle", Motion::Animation("idle".to_owned())),
            state(
                "move",
                Motion::Blend {
                    value: "speed".to_owned(),
                    points: vec![point("walk", 1.0), point("run", 3.0)],
                },
            ),
            state("jump", Motion::Animation("jump".to_owned())),
        ],
        transitions: vec![
            to_move,
            transition(
                Some("move"),
                "idle",
                vec![condition("speed", Test::Below(0.1))],
            ),
            transition(
                None,
                "jump",
                vec![
                    condition("jump", Test::Triggered),
                    condition("grounded", Test::Is(true)),
                ],
            ),
            landed,
        ],
    }
}

/// A hero and its animator, with `hero.anim` and `hero.states`.
fn hero() -> (Rc<TestNode>, Rc<TestNode>) {
    tracks::set_loader(|path| match path {
        "hero.anim" => Ok(animations()),
        _ => Err("it does not exist".to_owned()),
    });
    set_loader(|path| match path {
        "hero.states" => Ok(hero_machine()),
        _ => Err("it does not exist".to_owned()),
    });
    let hero = Rc::new(TestNode::new("Hero"));
    let mut animator = TestNode::new("Animator");
    animator.animator = Some(crate::node_classes::AnimatorFields::new());
    let animator = Rc::new(animator);
    nodes::add_child::<TestHost>(&hero, Rc::clone(&animator));
    if let Some(fields) = &animator.animator {
        fields.animations.set("hero.anim".to_owned());
        fields.states.set("hero.states".to_owned());
    }
    (hero, animator)
}

fn x_after(hero: &TestNode, animator: &Rc<TestNode>, delta: f32) -> f32 {
    tracks::advance::<TestHost>(animator, delta);
    hero.node.x.get()
}

#[test]
fn machines_start_in_their_first_state() {
    let (hero, animator) = hero();

    assert!((x_after(&hero, &animator, 0.1)).abs() < EPSILON);

    assert_eq!(state::<TestHost>(&animator), "idle");
    assert_eq!(tracks::animation::<TestHost>(&animator), "idle");
    assert!(tracks::is_playing::<TestHost>(&animator));
}

#[test]
fn transitions_fade_into_the_next_state() {
    let (hero, animator) = hero();
    x_after(&hero, &animator, 0.1);

    set_value::<TestHost>(&animator, "speed", 1.0);
    let halfway = [0.25, 0.25].map(|delta| x_after(&hero, &animator, delta));
    let faded = x_after(&hero, &animator, 0.25);

    assert_eq!(state::<TestHost>(&animator), "move");
    assert!(halfway[0].abs() < EPSILON, "{halfway:?}");
    assert!((halfway[1] - 5.0).abs() < EPSILON, "{halfway:?}");
    assert!((faded - 10.0).abs() < EPSILON, "{faded}");
}

#[test]
fn blends_mix_the_animations_around_their_value() {
    let (hero, animator) = hero();
    set_value::<TestHost>(&animator, "speed", 2.5);
    for _ in 0..4 {
        x_after(&hero, &animator, 0.25);
    }

    assert!((x_after(&hero, &animator, 0.25) - 17.5).abs() < EPSILON);
    assert_eq!(tracks::animation::<TestHost>(&animator), "run");
    set_value::<TestHost>(&animator, "speed", 9.0);
    assert!((x_after(&hero, &animator, 0.25) - 20.0).abs() < EPSILON);
}

#[test]
fn triggers_last_one_frame_and_conditions_must_all_hold() {
    let (hero, animator) = hero();
    x_after(&hero, &animator, 0.1);

    trigger::<TestHost>(&animator, "jump");
    x_after(&hero, &animator, 0.1);
    assert_eq!(state::<TestHost>(&animator), "idle", "it is not grounded");
    set_flag::<TestHost>(&animator, "grounded", true);
    x_after(&hero, &animator, 0.1);
    assert_eq!(state::<TestHost>(&animator), "idle", "the trigger was used");
    trigger::<TestHost>(&animator, "jump");
    x_after(&hero, &animator, 0.1);

    assert_eq!(state::<TestHost>(&animator), "jump");
}

#[test]
fn transitions_at_the_end_wait_and_animations_that_end_are_told() {
    let (hero, animator) = hero();
    set_flag::<TestHost>(&animator, "grounded", true);
    trigger::<TestHost>(&animator, "jump");
    x_after(&hero, &animator, 0.5);
    assert!((x_after(&hero, &animator, 0.5) - 50.0).abs() < EPSILON);

    assert!((x_after(&hero, &animator, 0.5) - 100.0).abs() < EPSILON);
    assert_eq!(animator.calls(), ["finished jump"]);
    assert_eq!(state::<TestHost>(&animator), "jump");
    x_after(&hero, &animator, 0.1);

    assert_eq!(state::<TestHost>(&animator), "idle");
    assert_eq!(animator.calls(), ["finished jump"]);
}

#[test]
fn play_jumps_to_a_state_and_stop_holds_the_machine() {
    let (hero, animator) = hero();
    x_after(&hero, &animator, 0.1);

    tracks::play::<TestHost>(&animator, "jump");
    x_after(&hero, &animator, 0.5);
    x_after(&hero, &animator, 0.5);
    tracks::stop::<TestHost>(&animator);
    let held = x_after(&hero, &animator, 0.5);

    assert!((held - 50.0).abs() < EPSILON, "{held}");
    assert!(!tracks::is_playing::<TestHost>(&animator));
    tracks::play::<TestHost>(&animator, "jump");
    assert!(tracks::is_playing::<TestHost>(&animator));
    tracks::play::<TestHost>(&animator, "fly");
    assert_eq!(state::<TestHost>(&animator), "jump");
}

#[test]
fn values_of_another_kind_or_name_are_not_set() {
    let (hero, animator) = hero();

    set_flag::<TestHost>(&animator, "speed", true);
    set_value::<TestHost>(&animator, "height", 4.0);
    x_after(&hero, &animator, 0.1);

    assert_eq!(state::<TestHost>(&animator), "idle");
}

#[test]
fn animators_say_which_clips_they_show_and_how_much() {
    let (hero, animator) = hero();
    x_after(&hero, &animator, 0.1);
    let alone: Vec<_> = tracks::shown_clips::<TestHost>(&animator)
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(alone.len(), 1);
    assert_eq!((alone[0].index, alone[0].weight), (0, 1.0), "idle, alone");

    set_value::<TestHost>(&animator, "speed", 1.0);
    x_after(&hero, &animator, 0.25);
    x_after(&hero, &animator, 0.25);
    let fading: Vec<_> = tracks::shown_clips::<TestHost>(&animator)
        .into_iter()
        .flatten()
        .collect();

    let indices: Vec<usize> = fading.iter().map(|clip| clip.index).collect();
    assert_eq!(indices, [0, 1], "from idle to walk");
    let total: f32 = fading.iter().map(|clip| clip.weight).sum();
    assert!((total - 1.0).abs() < EPSILON, "{fading:?}");
}
