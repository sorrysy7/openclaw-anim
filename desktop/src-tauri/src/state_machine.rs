use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Idle,
    Read,
    Write,
    Exec,
    WebSearch,
    WebFetch,
    Browser,
    Reply,
    Error,
}

#[derive(Debug, Clone)]
pub struct UiAnimEvent {
    pub action: Action,
    pub phase: &'static str, // "enter" only for MVP
    pub ts_ms: u64,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StateMachineConfig {
    pub min_hold: Duration,
    pub error_hold: Duration,
    /// Optional overrides: tool name -> action id (e.g. "feishu_doc" -> "write")
    pub tool_action_overrides: HashMap<String, String>,
}

pub struct StateMachine {
    cfg: StateMachineConfig,
    current: Action,
    last_change: Instant,
    // tool -> count (supports nested/overlapping)
    active: HashMap<String, u32>,
    // if we're in error overlay
    error_until: Option<Instant>,
    // delayed switch (anti-flicker)
    pending: Option<(Action, Instant)>,
}

impl StateMachine {
    pub fn new(cfg: StateMachineConfig) -> Self {
        Self {
            cfg,
            current: Action::Idle,
            last_change: Instant::now(),
            active: HashMap::new(),
            error_until: None,
            pending: None,
        }
    }

    pub fn current(&self) -> Action {
        self.current
    }

    fn action_from_id(id: &str) -> Option<Action> {
        match id {
            "idle" => Some(Action::Idle),
            "read" => Some(Action::Read),
            "write" => Some(Action::Write),
            "exec" => Some(Action::Exec),
            "web_search" => Some(Action::WebSearch),
            "web_fetch" => Some(Action::WebFetch),
            "browser" => Some(Action::Browser),
            "reply" => Some(Action::Reply),
            "error" => Some(Action::Error),
            _ => None,
        }
    }

    fn map_tool(&self, tool: &str) -> Action {
        if let Some(id) = self.cfg.tool_action_overrides.get(tool) {
            if let Some(a) = Self::action_from_id(id) {
                return a;
            }
        }

        match tool {
            "read" | "memory_get" | "memory_search" => Action::Read,
            "write" | "edit" | "feishu_doc" => Action::Write,
            "exec" | "process" | "gateway" => Action::Exec,
            "web_search" => Action::WebSearch,
            "web_fetch" => Action::WebFetch,
            "browser" => Action::Browser,
            "assistant_reply" => Action::Reply,
            _ => Action::Exec,
        }
    }

    fn priority(a: Action) -> u8 {
        match a {
            Action::Error => 100,
            Action::Reply => 90,
            Action::Exec | Action::Browser | Action::WebFetch | Action::WebSearch => 80,
            Action::Write => 60,
            Action::Read => 40,
            Action::Idle => 0,
        }
    }

    fn choose_action_from_active(&self) -> Action {
        // pick highest priority among active tools
        let mut best = Action::Idle;
        let mut best_p = 0u8;
        for (tool, count) in &self.active {
            if *count == 0 {
                continue;
            }
            let a = self.map_tool(tool);
            let p = Self::priority(a);
            if p > best_p {
                best = a;
                best_p = p;
            }
        }
        best
    }

    fn maybe_switch(&mut self, next: Action) -> Option<Action> {
        if next == self.current {
            self.pending = None;
            return None;
        }

        let now = Instant::now();
        let elapsed = self.last_change.elapsed();

        if elapsed < self.cfg.min_hold {
            // schedule a delayed switch at last_change + min_hold
            // If there is already a pending action, only override it when `next` has >= priority.
            let due = self.last_change + self.cfg.min_hold;
            match self.pending {
                Some((pending_action, pending_due)) => {
                    // keep the earliest due (they should be equal, but be defensive)
                    let due = std::cmp::min(due, pending_due);
                    if Self::priority(next) >= Self::priority(pending_action) {
                        self.pending = Some((next, due));
                    } else {
                        self.pending = Some((pending_action, due));
                    }
                }
                None => {
                    self.pending = Some((next, due));
                }
            }
            return None;
        }

        self.pending = None;
        self.current = next;
        self.last_change = now;
        Some(next)
    }

    /// Feed one gateway event. Returns an optional UI event if we decided to switch action.
    pub fn on_gateway_event(
        &mut self,
        ts_ms: u64,
        run_id: Option<String>,
        phase: &str,
        tool: &str,
    ) -> Option<UiAnimEvent> {
        // error overlay handling
        if let Some(until) = self.error_until {
            if Instant::now() < until {
                // ignore switches during error hold (but still track active tools)
            } else {
                self.error_until = None;
            }
        }

        // apply pending delayed switch if due (and not in error hold)
        if self.error_until.is_none() {
            if let Some((next, due)) = self.pending {
                if Instant::now() >= due {
                    self.pending = None;
                    if next != self.current {
                        self.current = next;
                        self.last_change = Instant::now();
                        return Some(UiAnimEvent {
                            action: next,
                            phase: "enter",
                            ts_ms,
                            run_id: run_id.clone(),
                        });
                    }
                }
            }
        }

        match phase {
            "start" => {
                *self.active.entry(tool.to_string()).or_insert(0) += 1;
                let desired = self.map_tool(tool);
                // reply is treated as a tool action too, but we also have explicit reply_start/reply_end
                if self.error_until.is_none() {
                    if let Some(switched) = self.maybe_switch(desired) {
                        return Some(UiAnimEvent {
                            action: switched,
                            phase: "enter",
                            ts_ms,
                            run_id,
                        });
                    }
                }
                None
            }
            "end" => {
                if let Some(v) = self.active.get_mut(tool) {
                    *v = v.saturating_sub(1);
                }
                if self.error_until.is_some() {
                    return None;
                }
                let next = self.choose_action_from_active();
                if let Some(switched) = self.maybe_switch(next) {
                    return Some(UiAnimEvent {
                        action: switched,
                        phase: "enter",
                        ts_ms,
                        run_id,
                    });
                }
                None
            }
            "error" => {
                // enter error state immediately, and hold
                self.error_until = Some(Instant::now() + self.cfg.error_hold);
                self.current = Action::Error;
                self.last_change = Instant::now();
                Some(UiAnimEvent {
                    action: Action::Error,
                    phase: "enter",
                    ts_ms,
                    run_id,
                })
            }
            "reply_start" => {
                // treat as reply action regardless of tool stack
                if self.error_until.is_some() {
                    return None;
                }
                if let Some(switched) = self.maybe_switch(Action::Reply) {
                    return Some(UiAnimEvent {
                        action: switched,
                        phase: "enter",
                        ts_ms,
                        run_id,
                    });
                }
                None
            }
            "reply_end" => {
                if self.error_until.is_some() {
                    return None;
                }
                let next = self.choose_action_from_active();
                if let Some(switched) = self.maybe_switch(next) {
                    return Some(UiAnimEvent {
                        action: switched,
                        phase: "enter",
                        ts_ms,
                        run_id,
                    });
                }
                None
            }
            _ => None,
        }
    }
}
