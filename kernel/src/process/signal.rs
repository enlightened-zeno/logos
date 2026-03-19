extern crate alloc;

use crate::sync::SpinLock;
use alloc::collections::BTreeMap;

/// POSIX signal numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
}

impl Signal {
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Signal::SIGHUP),
            2 => Some(Signal::SIGINT),
            3 => Some(Signal::SIGQUIT),
            4 => Some(Signal::SIGILL),
            5 => Some(Signal::SIGTRAP),
            6 => Some(Signal::SIGABRT),
            7 => Some(Signal::SIGBUS),
            8 => Some(Signal::SIGFPE),
            9 => Some(Signal::SIGKILL),
            10 => Some(Signal::SIGUSR1),
            11 => Some(Signal::SIGSEGV),
            12 => Some(Signal::SIGUSR2),
            13 => Some(Signal::SIGPIPE),
            14 => Some(Signal::SIGALRM),
            15 => Some(Signal::SIGTERM),
            17 => Some(Signal::SIGCHLD),
            18 => Some(Signal::SIGCONT),
            19 => Some(Signal::SIGSTOP),
            20 => Some(Signal::SIGTSTP),
            _ => None,
        }
    }

    /// Default action for this signal.
    pub fn default_action(self) -> SignalAction {
        match self {
            Signal::SIGHUP
            | Signal::SIGINT
            | Signal::SIGQUIT
            | Signal::SIGILL
            | Signal::SIGABRT
            | Signal::SIGBUS
            | Signal::SIGFPE
            | Signal::SIGKILL
            | Signal::SIGSEGV
            | Signal::SIGPIPE
            | Signal::SIGALRM
            | Signal::SIGTERM
            | Signal::SIGUSR1
            | Signal::SIGUSR2 => SignalAction::Terminate,
            Signal::SIGCHLD => SignalAction::Ignore,
            Signal::SIGCONT => SignalAction::Continue,
            Signal::SIGSTOP | Signal::SIGTSTP => SignalAction::Stop,
            Signal::SIGTRAP => SignalAction::Terminate,
        }
    }
}

/// What to do when a signal is received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAction {
    Terminate,
    Ignore,
    Stop,
    Continue,
}

/// Signal handler type: function pointer in user space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigHandler {
    /// Default action.
    Default,
    /// Ignore the signal.
    Ignore,
    /// User-space handler function address.
    Handler(u64),
}

/// Per-process signal state.
pub struct SignalState {
    /// Bitmask of pending signals.
    pub pending: u64,
    /// Bitmask of blocked (masked) signals.
    pub blocked: u64,
    /// Handlers for signals 1-31.
    pub handlers: [SigHandler; 32],
}

impl SignalState {
    pub const fn new() -> Self {
        Self {
            pending: 0,
            blocked: 0,
            handlers: [SigHandler::Default; 32],
        }
    }

    /// Set a handler for a signal.
    pub fn set_handler(&mut self, sig: Signal, handler: SigHandler) {
        let idx = sig as usize;
        if idx < 32 {
            // SIGKILL and SIGSTOP cannot be caught
            if sig != Signal::SIGKILL && sig != Signal::SIGSTOP {
                self.handlers[idx] = handler;
            }
        }
    }

    /// Get the handler for a signal.
    pub fn get_handler(&self, sig: Signal) -> SigHandler {
        let idx = sig as usize;
        if idx < 32 {
            self.handlers[idx]
        } else {
            SigHandler::Default
        }
    }

    /// Queue a signal.
    pub fn send(&mut self, sig: Signal) {
        self.pending |= 1 << (sig as u8);
    }

    /// Check if any unblocked signal is pending.
    pub fn has_pending(&self) -> bool {
        (self.pending & !self.blocked) != 0
    }

    /// Dequeue the highest-priority pending unblocked signal.
    pub fn dequeue(&mut self) -> Option<Signal> {
        let deliverable = self.pending & !self.blocked;
        if deliverable == 0 {
            return None;
        }

        // Find lowest set bit
        let bit = deliverable.trailing_zeros() as u8;
        self.pending &= !(1 << bit);
        Signal::from_number(bit)
    }
}

/// Global per-process signal state store.
static SIGNAL_STATES: SpinLock<Option<BTreeMap<u64, SignalState>>> = SpinLock::new(None);

/// Initialize the signal state store.
pub fn init() {
    let mut states = BTreeMap::new();
    states.insert(1, SignalState::new()); // PID 1
    *SIGNAL_STATES.lock() = Some(states);
}

/// Create signal state for a new process.
pub fn create_for_pid(pid: u64) {
    let mut guard = SIGNAL_STATES.lock();
    if let Some(states) = guard.as_mut() {
        states.insert(pid, SignalState::new());
    }
}

/// Remove signal state for an exited process.
pub fn remove_for_pid(pid: u64) {
    let mut guard = SIGNAL_STATES.lock();
    if let Some(states) = guard.as_mut() {
        states.remove(&pid);
    }
}

/// Run a closure with the signal state of the given PID.
pub fn with_signal_state<F, R>(pid: u64, f: F) -> Option<R>
where
    F: FnOnce(&mut SignalState) -> R,
{
    let mut guard = SIGNAL_STATES.lock();
    let states = guard.as_mut()?;
    let state = states.get_mut(&pid)?;
    Some(f(state))
}

/// Send a signal to a process.
pub fn send_signal(pid: u64, sig: Signal) -> bool {
    with_signal_state(pid, |state| {
        state.send(sig);
    })
    .is_some()
}
