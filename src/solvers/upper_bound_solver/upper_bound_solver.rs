use crate::{
    game::{
        state::InProgress, units::*, Action, ActionMask, ComboAction, Condition, Effects, Settings,
        State,
    },
    solvers::actions::{MIXED_ACTIONS, PROGRESS_ACTIONS, QUALITY_ACTIONS},
};

use rustc_hash::FxHashMap as HashMap;

use super::pareto_front::{ParetoFrontBuilder, ParetoValue};

const SEARCH_ACTIONS: ActionMask = PROGRESS_ACTIONS.union(QUALITY_ACTIONS).union(MIXED_ACTIONS);

const INF_PROGRESS: Progress = Progress::new(100_000);
const INF_QUALITY: Quality = Quality::new(100_000);
const INF_DURABILITY: Durability = 100;

pub const WASTE_NOT_COST: CP = Action::WasteNot2.base_cp_cost() / 8;
pub const MANIPULATION_COST: CP = Action::Manipulation.base_cp_cost() / 8;
// CP value for 5 Durability
pub const DURABILITY_COST: CP = Action::Manipulation.base_cp_cost() / 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ReducedEffects {
    pub inner_quiet: u8,
    pub innovation: u8,
    pub veneration: u8,
    pub great_strides: u8,
    pub muscle_memory: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ReducedState {
    cp: CP,
    combo: Option<ComboAction>,
    effects: ReducedEffects,
}

impl std::convert::From<InProgress> for ReducedState {
    fn from(state: InProgress) -> Self {
        let used_durability = (INF_DURABILITY - state.durability) / 5;
        let durability_cost = std::cmp::min(
            used_durability * DURABILITY_COST,
            (used_durability + 1) / 2 * DURABILITY_COST + WASTE_NOT_COST,
        );
        Self {
            cp: state.cp - durability_cost,
            combo: state.combo,
            effects: ReducedEffects {
                inner_quiet: state.effects.inner_quiet,
                innovation: state.effects.innovation,
                veneration: state.effects.veneration,
                great_strides: state.effects.great_strides,
                muscle_memory: state.effects.muscle_memory,
            },
        }
    }
}

impl std::convert::From<ReducedState> for InProgress {
    fn from(state: ReducedState) -> Self {
        Self {
            durability: INF_DURABILITY,
            cp: state.cp,
            missing_progress: INF_PROGRESS,
            missing_quality: INF_QUALITY,
            effects: Effects {
                inner_quiet: state.effects.inner_quiet,
                waste_not: 0,
                innovation: state.effects.innovation,
                veneration: state.effects.veneration,
                great_strides: state.effects.great_strides,
                muscle_memory: state.effects.muscle_memory,
                manipulation: 0,
            },
            combo: state.combo,
        }
    }
}

pub struct UpperBoundSolver {
    settings: Settings,
    solved_states: HashMap<ReducedState, Box<[ParetoValue]>>,
    pareto_front_builder: ParetoFrontBuilder,
}

impl UpperBoundSolver {
    pub fn new(settings: Settings) -> Self {
        dbg!(std::mem::size_of::<ReducedState>());
        dbg!(std::mem::align_of::<ReducedState>());
        UpperBoundSolver {
            settings,
            solved_states: HashMap::default(),
            pareto_front_builder: ParetoFrontBuilder::new(settings),
        }
    }

    pub fn quality_upper_bound(&mut self, mut state: InProgress) -> Quality {
        let current_quality = self.settings.max_quality.sub(state.missing_quality);

        // refund effects and durability
        state.cp += state.effects.manipulation as CP * MANIPULATION_COST;
        state.cp += state.effects.waste_not as CP * WASTE_NOT_COST;
        state.cp += state.durability / 5 * DURABILITY_COST;
        state.durability = INF_DURABILITY;

        let reduced_state = ReducedState::from(state);

        if !self.solved_states.contains_key(&reduced_state) {
            self.solve_state(reduced_state);
            self.pareto_front_builder.clear();
        }
        let pareto_front = self.solved_states.get(&reduced_state).unwrap();

        match pareto_front.first() {
            Some(first) => {
                if first.progress < state.missing_progress {
                    return Quality::new(0);
                }
            }
            None => return Quality::new(0),
        }

        let mut lo = 0;
        let mut hi = pareto_front.len();
        while lo + 1 != hi {
            let m = (lo + hi) / 2;
            if pareto_front[m].progress < state.missing_progress {
                hi = m;
            } else {
                lo = m;
            }
        }

        std::cmp::min(
            self.settings.max_quality,
            pareto_front[lo].quality.add(current_quality),
        )
    }

    fn solve_state(&mut self, state: ReducedState) {
        match state.combo {
            Some(combo) => {
                let non_combo_state = ReducedState {
                    combo: None,
                    ..state
                };
                match self.solved_states.get(&non_combo_state) {
                    Some(pareto_front) => self.pareto_front_builder.push(&pareto_front),
                    None => self.solve_non_combo_state(non_combo_state),
                }
                let combo_actions: &[Action] = match combo {
                    ComboAction::SynthesisBegin => &[Action::MuscleMemory, Action::Reflect],
                    ComboAction::Observe => &[Action::FocusedSynthesis, Action::FocusedTouch],
                    ComboAction::BasicTouch => &[Action::ComboStandardTouch],
                    ComboAction::StandardTouch => &[Action::ComboAdvancedTouch],
                };
                for action in combo_actions {
                    if self.settings.allowed_actions.has(*action) {
                        self.build_child_front(state, *action);
                    }
                }
            }
            None => {
                self.solve_non_combo_state(state);
            }
        }
        let pareto_front = self.pareto_front_builder.peek().unwrap();
        self.solved_states.insert(state, pareto_front);
    }

    fn solve_non_combo_state(&mut self, state: ReducedState) {
        self.pareto_front_builder.push_empty();
        for action in SEARCH_ACTIONS
            .intersection(self.settings.allowed_actions)
            .actions_iter()
        {
            self.build_child_front(state, action);
        }
    }

    fn build_child_front(&mut self, state: ReducedState, action: Action) {
        let new_state =
            InProgress::from(state).use_action(action, Condition::Normal, &self.settings);
        match new_state {
            State::InProgress(new_state) => {
                let action_progress = INF_PROGRESS.sub(new_state.missing_progress);
                let action_quality = INF_QUALITY.sub(new_state.missing_quality);
                let new_state = ReducedState::from(new_state);
                if new_state.cp > 0 {
                    match self.solved_states.get(&new_state) {
                        Some(pareto_front) => self.pareto_front_builder.push(&pareto_front),
                        None => self.solve_state(new_state),
                    }
                    self.pareto_front_builder
                        .add(action_progress, action_quality);
                    self.pareto_front_builder.merge();
                } else if new_state.cp + DURABILITY_COST >= 0 && action_progress != Progress::new(0)
                {
                    // "durability" must not go lower than -5
                    // last action must be a progress increase
                    self.pareto_front_builder
                        .push(&[ParetoValue::new(Progress::new(0), Quality::new(0))]);
                    self.pareto_front_builder
                        .add(action_progress, action_quality);
                    self.pareto_front_builder.merge();
                }
            }
            State::Invalid => (),
            _ => unreachable!("INF_PROGRESS or INF_DURABILITY not high enough"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::*;

    fn solve(settings: Settings, actions: &[Action]) -> f32 {
        let state = State::new(&settings).use_actions(actions, Condition::Normal, &settings);
        let result = UpperBoundSolver::new(settings)
            .quality_upper_bound(state.as_in_progress().unwrap())
            .into();
        dbg!(result);
        result
    }

    #[test]
    fn test_01() {
        let settings = Settings {
            max_cp: 553,
            max_durability: 70,
            max_progress: Progress::from(2400.00),
            max_quality: Quality::from(20000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(
            settings,
            &[
                Action::MuscleMemory,
                Action::PrudentTouch,
                Action::Manipulation,
                Action::Veneration,
                Action::WasteNot2,
                Action::Groundwork,
                Action::Groundwork,
                Action::Groundwork,
                Action::PreparatoryTouch,
            ],
        );
        assert_eq!(result, 3485.00); // tightness test
        assert_ge!(result, 3352.50); // correctness test
    }

    #[test]
    fn test_02() {
        let settings = Settings {
            max_cp: 700,
            max_durability: 70,
            max_progress: Progress::from(2500.00),
            max_quality: Quality::from(5000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(
            settings,
            &[
                Action::MuscleMemory,
                Action::Manipulation,
                Action::Veneration,
                Action::WasteNot,
                Action::Groundwork,
                Action::Groundwork,
            ],
        );
        assert_eq!(result, 4767.50); // tightness test
        assert_ge!(result, 4685.00); // correctness test
    }

    #[test]
    fn test_03() {
        let settings = Settings {
            max_cp: 617,
            max_durability: 60,
            max_progress: Progress::from(2120.00),
            max_quality: Quality::from(5000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(
            settings,
            &[
                Action::MuscleMemory,
                Action::Manipulation,
                Action::Veneration,
                Action::WasteNot,
                Action::Groundwork,
                Action::CarefulSynthesis,
                Action::Groundwork,
                Action::PreparatoryTouch,
                Action::Innovation,
                Action::BasicTouch,
                Action::ComboStandardTouch,
            ],
        );
        assert_eq!(result, 4055.00); // tightness test
        assert_ge!(result, 4055.00); // correctness test
    }

    #[test]
    fn test_04() {
        let settings = Settings {
            max_cp: 411,
            max_durability: 60,
            max_progress: Progress::from(1990.00),
            max_quality: Quality::from(5000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(settings, &[Action::MuscleMemory]);
        assert_eq!(result, 2221.25); // tightness test
        assert_ge!(result, 2011.25); // correctness test
    }

    #[test]
    fn test_05() {
        let settings = Settings {
            max_cp: 450,
            max_durability: 60,
            max_progress: Progress::from(1970.00),
            max_quality: Quality::from(2000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(settings, &[Action::MuscleMemory]);
        assert_eq!(result, 2000.00); // tightness test
        assert_ge!(result, 2000.00); // correctness test
    }

    #[test]
    fn test_06() {
        let settings = Settings {
            max_cp: 673,
            max_durability: 60,
            max_progress: Progress::from(2345.00),
            max_quality: Quality::from(8000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(settings, &[Action::MuscleMemory]);
        assert_eq!(result, 4556.25); // tightness test
        assert_ge!(result, 4405.00); // correctness test
    }

    #[test]
    fn test_07() {
        let settings = Settings {
            max_cp: 673,
            max_durability: 60,
            max_progress: Progress::from(2345.00),
            max_quality: Quality::from(8000.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(settings, &[Action::Reflect]);
        assert_eq!(result, 4477.50); // tightness test
        assert_ge!(result, 4138.75); // correctness test
    }

    #[test]
    fn test_08() {
        let settings = Settings {
            max_cp: 32,
            max_durability: 10,
            max_progress: Progress::from(100.00),
            max_quality: Quality::from(200.00),
            allowed_actions: ActionMask::from_level(90, true),
        };
        let result = solve(settings, &[Action::PrudentTouch]);
        assert_eq!(result, 100.00); // tightness test
        assert_ge!(result, 100.00); // correctness test
    }
}
