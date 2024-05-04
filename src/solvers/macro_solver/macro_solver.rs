use crate::game::{state::InProgress, units::Quality, Action, Settings, State};
use crate::solvers::{FinishSolver, UpperBoundSolver};

use std::time::Instant;
use std::vec::Vec;
use typed_arena::Arena;

use strum::IntoEnumIterator;

use super::*;

pub struct MacroSolver {
    settings: Settings,
    finish_solver: FinishSolver,
    bound_solver: UpperBoundSolver,
}

impl MacroSolver {
    pub fn new(settings: Settings) -> MacroSolver {
        MacroSolver {
            settings,
            finish_solver: FinishSolver::new(settings),
            bound_solver: UpperBoundSolver::new(settings),
        }
    }

    pub fn solve(&mut self, state: State) -> Option<Vec<Action>> {
        match state {
            State::InProgress(state) => self.do_solve(state),
            _ => None,
        }
    }

    fn do_solve(&mut self, state: InProgress) -> Option<Vec<Action>> {
        let timer = Instant::now();
        let mut finish_solver_rejected_node: usize = 0;
        let mut upper_bound_solver_rejected_nodes: usize = 0;

        let traces: Arena<Option<SearchTrace>> = Arena::new();
        let mut search_queue = SearchQueue::new(self.settings);

        search_queue.push(SearchNode { state, trace: None });

        let mut best_quality = Quality::new(0);
        let mut best_actions: Option<Vec<Action>> = None;

        while let Some(current_node) = search_queue.pop() {
            let trace: &Option<SearchTrace> = traces.alloc(current_node.trace);
            for sequence in ActionSequence::iter() {
                if !sequence.should_use(&current_node.state, &self.settings) {
                    continue;
                }
                let new_state =
                    sequence.apply(State::InProgress(current_node.state), &self.settings);
                if let State::InProgress(state) = new_state {
                    if !self.finish_solver.can_finish(&state) {
                        finish_solver_rejected_node += 1;
                        continue;
                    }
                    if self.bound_solver.quality_upper_bound(&state) <= best_quality {
                        upper_bound_solver_rejected_nodes += 1;
                        continue;
                    }
                    let final_quality = self
                        .settings
                        .max_quality
                        .saturating_sub(state.missing_quality);
                    if final_quality > best_quality {
                        best_quality = final_quality;
                        let mut actions = SearchTrace::new(trace, sequence).actions();
                        actions.extend(self.finish_solver.get_finish_sequence(&state).unwrap());
                        best_actions = Some(actions);
                    }
                    search_queue.push(SearchNode {
                        state,
                        trace: Some(SearchTrace::new(trace, sequence)),
                    });
                }
            }
        }

        let seconds = timer.elapsed().as_secs_f32();
        let nodes = traces.len();
        let nodes_per_sec = nodes as f32 / seconds;
        dbg!(seconds, nodes, nodes_per_sec);

        dbg!(
            finish_solver_rejected_node,
            upper_bound_solver_rejected_nodes
        );

        dbg!(f32::from(best_quality));
        best_actions
    }
}
