use crate::paths::AppPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub text: String,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Put(Note),
    Delete(String),
}
impl Op {
    fn id(&self) -> &str {
        match self {
            Self::Put(note) => &note.id,
            Self::Delete(id) => id,
        }
    }
    fn apply(&self, notes: &mut Vec<Note>) {
        notes.retain(|note| note.id != self.id());
        if let Self::Put(note) = self {
            notes.push(note.clone());
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Store {
    pub notes: Vec<Note>,
    pub pending: Vec<Op>,
}
impl Store {
    pub fn load(paths: &AppPaths) -> Result<Self> {
        let path = paths.data_dir.join("notes.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        serde_json::from_slice(&std::fs::read(path)?).context("Could not read notes")
    }
    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        std::fs::create_dir_all(&paths.data_dir)?;
        let path = paths.data_dir.join("notes.json");
        let temp = paths.data_dir.join("notes.json.tmp");
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&serde_json::to_vec(self)?)?;
        file.sync_all()?;
        std::fs::rename(temp, path)?;
        Ok(())
    }
    pub fn change(&mut self, op: Op) {
        op.apply(&mut self.notes);
        self.pending.retain(|pending| pending.id() != op.id());
        self.pending.push(op);
        self.sort();
    }
    fn sort(&mut self) {
        self.notes
            .sort_by_key(|note| std::cmp::Reverse(note.created_at));
    }
    pub fn complete(&mut self, sent: &[Op], remote: Option<Vec<Note>>) {
        for op in sent {
            if let Some(index) = self.pending.iter().position(|pending| pending == op) {
                self.pending.remove(index);
            }
        }
        if let Some(remote) = remote {
            self.notes = remote;
            for op in &self.pending {
                op.apply(&mut self.notes);
            }
            self.sort();
        }
    }
}

pub fn new_note(text: String) -> Result<Note> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("Could not generate note ID"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let id = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    let now = chrono::Utc::now().timestamp_millis();
    Ok(Note {
        id,
        text,
        pinned: false,
        created_at: now,
        updated_at: now,
    })
}

pub fn synchronize(pending: Vec<Op>, url: &str, token: &str) -> (Vec<Op>, Option<Vec<Note>>) {
    #[derive(Deserialize)]
    struct Response {
        notes: Vec<Note>,
    }
    let client = crate::http::client();
    let endpoint = format!("{url}/v1/notes");
    let mut sent = Vec::new();
    for op in pending {
        let request = match &op {
            Op::Put(note) => client.put(&endpoint).json(note),
            Op::Delete(id) => client.delete(&endpoint).query(&[("id", id)]),
        };
        match request.bearer_auth(token).send() {
            Ok(response)
                if response.status().is_success()
                    || matches!(response.status().as_u16(), 400 | 404) =>
            {
                sent.push(op)
            }
            _ => break,
        }
    }
    let remote = client
        .get(endpoint)
        .bearer_auth(token)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json::<Response>())
        .ok()
        .map(|response| response.notes);
    (sent, remote)
}
