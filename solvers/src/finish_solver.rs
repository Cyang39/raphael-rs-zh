use simulator::{
    state::InProgress, Action, ActionMask, ComboAction, Condition, Effects, Settings,
    SimulationState,
};

use rustc_hash::FxHashMap as HashMap;

use super::{
    actions::{DURABILITY_ACTIONS, PROGRESS_ACTIONS},
    pareto_front::{ParetoFrontBuilder, ParetoValue},
};

const INF_PROGRESS: u32 = 1_000_000;
const SEARCH_ACTIONS: ActionMask = PROGRESS_ACTIONS.union(DURABILITY_ACTIONS);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ReducedEffects {
    pub muscle_memory: u8,
    pub waste_not: u8,
    pub veneration: u8,
    pub manipulation: u8,
}

impl ReducedEffects {
    pub fn from_effects(effects: &Effects) -> ReducedEffects {
        ReducedEffects {
            muscle_memory: effects.muscle_memory,
            waste_not: effects.waste_not,
            veneration: effects.veneration,
            manipulation: effects.manipulation,
        }
    }

    pub fn to_effects(self) -> Effects {
        Effects {
            inner_quiet: 0,
            waste_not: self.waste_not,
            innovation: 0,
            veneration: self.veneration,
            great_strides: 0,
            muscle_memory: self.muscle_memory,
            manipulation: self.manipulation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ReducedState {
    durability: i16,
    cp: i16,
    effects: ReducedEffects,
    combo: Option<ComboAction>,
    trained_perfection_used: bool,
    trained_perfection_active: bool,
}

impl ReducedState {
    pub fn from_state(state: &InProgress) -> ReducedState {
        ReducedState {
            durability: state.raw_state().durability,
            cp: state.raw_state().cp,
            effects: ReducedEffects::from_effects(&state.raw_state().effects),
            combo: state.raw_state().combo,
            trained_perfection_used: state.raw_state().trained_perfection_used,
            trained_perfection_active: state.raw_state().trained_perfection_active,
        }
    }

    pub fn to_state(self) -> InProgress {
        let raw_state = SimulationState {
            durability: self.durability,
            cp: self.cp,
            missing_progress: INF_PROGRESS,
            missing_quality: 0,
            effects: self.effects.to_effects(),
            combo: self.combo,
            trained_perfection_used: self.trained_perfection_used,
            trained_perfection_active: self.trained_perfection_active,
        };
        raw_state.try_into().unwrap()
    }
}

pub struct FinishSolver {
    settings: Settings,
    // maximum attainable progress for each state
    max_progress: HashMap<ReducedState, u32>,
    // pareto-optimal set of (progress, duration) for each state
    pareto_fronts: HashMap<ReducedState, Box<[ParetoValue<u32, i32>]>>,
    pareto_front_builder: ParetoFrontBuilder<u32, i32>,
}

impl FinishSolver {
    pub fn new(settings: Settings) -> FinishSolver {
        dbg!(std::mem::size_of::<ReducedState>());
        dbg!(std::mem::align_of::<ReducedState>());
        FinishSolver {
            settings,
            max_progress: HashMap::default(),
            pareto_fronts: HashMap::default(),
            pareto_front_builder: ParetoFrontBuilder::new(settings.max_progress),
        }
    }

    pub fn get_finish_sequence(&mut self, mut state: InProgress) -> Option<Vec<Action>> {
        if !self.can_finish(&state) {
            return None;
        }
        let mut finish_sequence: Vec<Action> = Vec::new();
        loop {
            let mut best_time = i32::MAX;
            let mut best_action = Action::BasicSynthesis;
            for action in SEARCH_ACTIONS
                .intersection(self.settings.allowed_actions)
                .actions_iter()
            {
                if let Ok(new_state) = state.use_action(action, Condition::Normal, &self.settings) {
                    if let Ok(new_state) = new_state.try_into() {
                        let time = self.time_to_finish(&new_state);
                        if time.is_some() && time.unwrap() + action.time_cost() < best_time {
                            best_time = time.unwrap() + action.time_cost();
                            best_action = action;
                        }
                    } else if new_state.missing_progress == 0 {
                        if action.time_cost() < best_time {
                            best_time = action.time_cost();
                            best_action = action;
                        }
                    }
                }
            }

            finish_sequence.push(best_action);
            let new_state = state
                .use_action(best_action, Condition::Normal, &self.settings)
                .unwrap();
            if let Ok(new_state) = new_state.try_into() {
                state = new_state;
            } else {
                return Some(finish_sequence);
            }
        }
    }

    pub fn can_finish(&mut self, state: &InProgress) -> bool {
        let max_progress = self.solve_max_progress(ReducedState::from_state(state));
        max_progress >= state.raw_state().missing_progress
    }

    pub fn time_to_finish(&mut self, state: &InProgress) -> Option<i32> {
        let reduced_state = ReducedState::from_state(state);
        if !self.pareto_fronts.contains_key(&reduced_state) {
            self.solve_pareto_front(reduced_state);
            self.pareto_front_builder.clear();
        }
        let result: &[ParetoValue<u32, i32>] = self.pareto_fronts.get(&reduced_state).unwrap();
        for ParetoValue {
            first: progress,
            second: time,
        } in result.iter().rev()
        {
            if *progress >= state.raw_state().missing_progress {
                return Some(-*time);
            }
        }
        None
    }

    fn solve_max_progress(&mut self, state: ReducedState) -> u32 {
        match self.max_progress.get(&state) {
            Some(max_progress) => *max_progress,
            None => {
                let mut max_progress = 0;
                for action in SEARCH_ACTIONS
                    .intersection(self.settings.allowed_actions)
                    .actions_iter()
                {
                    if let Ok(new_state) =
                        state
                            .to_state()
                            .use_action(action, Condition::Normal, &self.settings)
                    {
                        if let Ok(in_progress) = new_state.try_into() {
                            let child_progress =
                                self.solve_max_progress(ReducedState::from_state(&in_progress));
                            let action_progress =
                                INF_PROGRESS - in_progress.raw_state().missing_progress;
                            max_progress =
                                std::cmp::max(max_progress, child_progress + action_progress);
                        } else {
                            let progress = INF_PROGRESS - new_state.missing_progress;
                            max_progress = std::cmp::max(max_progress, progress);
                        }
                    }
                }
                self.max_progress.insert(state, max_progress);
                max_progress
            }
        }
    }

    fn solve_pareto_front(&mut self, state: ReducedState) {
        self.pareto_front_builder.push_empty();
        for action in SEARCH_ACTIONS
            .intersection(self.settings.allowed_actions)
            .actions_iter()
        {
            if let Ok(new_state) =
                state
                    .to_state()
                    .use_action(action, Condition::Normal, &self.settings)
            {
                if let Ok(in_progress) = new_state.try_into() {
                    let progress = INF_PROGRESS - new_state.missing_progress;
                    match self
                        .pareto_fronts
                        .get(&ReducedState::from_state(&in_progress))
                    {
                        Some(pareto_front) => self.pareto_front_builder.push(pareto_front),
                        None => self.solve_pareto_front(ReducedState::from_state(&in_progress)),
                    }
                    self.pareto_front_builder.add(progress, -action.time_cost());
                    self.pareto_front_builder.merge();
                } else {
                    let progress = INF_PROGRESS - new_state.missing_progress;
                    self.pareto_front_builder
                        .push(&[ParetoValue::new(progress, -action.time_cost())]);
                    self.pareto_front_builder.merge();
                }
            }
        }
        let pareto_front = self.pareto_front_builder.peek().unwrap();
        self.pareto_fronts.insert(state, pareto_front);
    }
}

impl Drop for FinishSolver {
    fn drop(&mut self) {
        let simple_states = self.max_progress.len();
        let pareto_states = self.pareto_fronts.len();
        dbg!(simple_states, pareto_states);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solve(settings: Settings, actions: &[Action]) -> Vec<Action> {
        let state = SimulationState::from_macro(&settings, actions).unwrap();
        let result = FinishSolver::new(settings)
            .get_finish_sequence(state.try_into().unwrap())
            .unwrap();
        dbg!(&result);
        result
    }

    #[test]
    fn test_01() {
        let settings = Settings {
            max_cp: 100,
            max_durability: 30,
            max_progress: 360,
            max_quality: 20000,
            base_progress: 100,
            base_quality: 100,
            initial_quality: 0,
            job_level: 90,
            allowed_actions: ActionMask::from_level(90, true),
        };
        let actions = solve(settings, &[]);
        assert_eq!(actions, [Action::Groundwork]);
    }

    #[test]
    fn test_02() {
        let settings = Settings {
            max_cp: 100,
            max_durability: 60,
            max_progress: 2100,
            max_quality: 20000,
            base_progress: 100,
            base_quality: 100,
            initial_quality: 0,
            job_level: 90,
            allowed_actions: ActionMask::from_level(90, true),
        };
        let actions = solve(settings, &[]);
        assert_eq!(
            actions,
            [
                Action::MuscleMemory,
                Action::Veneration,
                Action::Groundwork,
                Action::Groundwork,
                Action::PrudentSynthesis,
                Action::BasicSynthesis,
            ]
        );
    }
}
