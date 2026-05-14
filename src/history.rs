use std::collections::VecDeque;

const CAP: usize = 64;

#[derive(Default)]
pub struct History {
    entries: VecDeque<String>,
}

impl History {
    pub fn record(&mut self, cmd: &str) {
        if self.entries.len() == CAP {
            self.entries.pop_front();
        }
        self.entries.push_back(cmd.to_string());
    }

    pub fn last(&self) -> Option<&str> {
        self.entries.back().map(String::as_str)
    }
}
