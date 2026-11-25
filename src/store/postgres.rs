//! PostgreSQL adapters for workspace, collaboration and auth (spec-137).
//!
//! Enabled with `--features postgres`; selected at startup through
//! `ION_OF_NOTES_POSTGRES_URL`. The schema mirrors the SQLite tables, including
//! page rows without blocks and one `workspace_blocks` row per block.

use crate::auth::{Account, AuthStore, Session};
use crate::collab::CollabState;
use crate::model::{
    Block, CommentDto, DataSource, Database, FileUpload, Page, View, Workspace, WorkspaceSettings,
    now_iso,
};
use crate::store::{CollabStore, SnapshotStore, WorkspaceStore};
use postgres::{Client, NoTls, Transaction};
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::{Arc, Mutex};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS workspace_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS workspace_pages (
    id TEXT PRIMARY KEY,
    position BIGINT NOT NULL,
    json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS workspace_pages_position ON workspace_pages(position);
CREATE TABLE IF NOT EXISTS workspace_blocks (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL,
    parent_id TEXT,
    position BIGINT NOT NULL,
    json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS workspace_blocks_parent
    ON workspace_blocks(page_id, parent_id, position);
CREATE TABLE IF NOT EXISTS collab (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    member_id TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    member_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_member ON sessions(member_id);
CREATE INDEX IF NOT EXISTS sessions_expires_at ON sessions(expires_at);
";

/// PostgreSQL implementation of the persistence and auth ports.
#[derive(Clone)]
pub struct PostgresStore {
    connection: Arc<Mutex<Client>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WorkspaceMeta {
    name: String,
    version: u64,
    settings: WorkspaceSettings,
    #[serde(default)]
    databases: Vec<Database>,
    #[serde(default)]
    data_sources: Vec<DataSource>,
    #[serde(default)]
    views: Vec<View>,
    #[serde(default)]
    comments: Vec<CommentDto>,
    #[serde(default)]
    file_uploads: Vec<FileUpload>,
}

impl WorkspaceMeta {
    fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            name: workspace.name.clone(),
            version: workspace.version,
            settings: workspace.settings.clone(),
            databases: workspace.databases.clone(),
            data_sources: workspace.data_sources.clone(),
            views: workspace.views.clone(),
            comments: workspace.comments.clone(),
            file_uploads: workspace.file_uploads.clone(),
        }
    }

    fn into_workspace(self, pages: Vec<Page>) -> Workspace {
        Workspace::from_stored_parts(
            self.name,
            pages,
            self.version,
            self.settings,
            self.databases,
            self.data_sources,
            self.views,
            self.comments,
            self.file_uploads,
        )
    }
}

impl PostgresStore {
    /// Connect and ensure the schema.
    pub fn open(url: &str) -> io::Result<Self> {
        let mut client = Client::connect(url, NoTls).map_err(io::Error::other)?;
        client.batch_execute(SCHEMA).map_err(io::Error::other)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(client)),
        })
    }

    /// Load the workspace snapshot, if present.
    pub fn load_workspace(&self) -> io::Result<Option<Workspace>> {
        let mut client = self.lock();
        let row = client
            .query_opt("SELECT json FROM workspace_meta WHERE id = 1", &[])
            .map_err(io::Error::other)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let meta_json: String = row.get(0);
        let meta: WorkspaceMeta = serde_json::from_str(&meta_json)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let mut pages = read_pages(&mut client)?;
        let mut blocks = read_blocks(&mut client)?;
        for page in &mut pages {
            attach_blocks(page, blocks.remove(&page.id).unwrap_or_default());
        }
        Ok(Some(meta.into_workspace(pages)))
    }

    /// Persist one workspace snapshot.
    pub fn persist_workspace(&self, workspace: &Workspace) -> io::Result<()> {
        let mut client = self.lock();
        let mut transaction = client.transaction().map_err(io::Error::other)?;
        write_meta(&mut transaction, &WorkspaceMeta::from_workspace(workspace))?;
        write_all_pages(&mut transaction, workspace)?;
        transaction.commit().map_err(io::Error::other)
    }

    /// Load the collaboration document, if present.
    pub fn load_collab(&self) -> io::Result<Option<CollabState>> {
        let mut client = self.lock();
        let row = client
            .query_opt("SELECT json FROM collab WHERE id = 1", &[])
            .map_err(io::Error::other)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let json: String = row.get(0);
        serde_json::from_str(&json)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    /// Persist one collaboration snapshot.
    pub fn persist_collab(&self, state: &CollabState) -> io::Result<()> {
        let mut client = self.lock();
        let mut transaction = client.transaction().map_err(io::Error::other)?;
        write_collab(&mut transaction, state)?;
        transaction.commit().map_err(io::Error::other)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Client> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl WorkspaceStore for PostgresStore {
    fn persist(&self, workspace: &Workspace) -> io::Result<()> {
        self.persist_workspace(workspace)
    }

    fn load(&self) -> io::Result<Option<Workspace>> {
        self.load_workspace()
    }
}

impl CollabStore for PostgresStore {
    fn persist(&self, state: &CollabState) -> io::Result<()> {
        self.persist_collab(state)
    }

    fn load(&self) -> io::Result<Option<CollabState>> {
        self.load_collab()
    }
}

impl SnapshotStore for PostgresStore {
    fn persist_snapshots(&self, workspace: &Workspace, collab: &CollabState) -> io::Result<()> {
        let mut client = self.lock();
        let mut transaction = client.transaction().map_err(io::Error::other)?;
        write_meta(&mut transaction, &WorkspaceMeta::from_workspace(workspace))?;
        write_all_pages(&mut transaction, workspace)?;
        write_collab(&mut transaction, collab)?;
        transaction.commit().map_err(io::Error::other)
    }

    fn persist_page(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
    ) -> io::Result<()> {
        let mut client = self.lock();
        let mut transaction = client.transaction().map_err(io::Error::other)?;
        write_meta(&mut transaction, &WorkspaceMeta::from_workspace(workspace))?;
        match workspace.pages.iter().position(|page| page.id == page_id) {
            Some(position) => {
                write_page(&mut transaction, &workspace.pages[position], position)?;
                transaction
                    .execute(
                        "DELETE FROM workspace_blocks WHERE page_id = $1",
                        &[&page_id],
                    )
                    .map_err(io::Error::other)?;
                write_blocks_recursive(
                    &mut transaction,
                    page_id,
                    None,
                    &workspace.pages[position].blocks,
                )?;
            }
            None => {
                transaction
                    .execute(
                        "DELETE FROM workspace_blocks WHERE page_id = $1",
                        &[&page_id],
                    )
                    .map_err(io::Error::other)?;
                transaction
                    .execute("DELETE FROM workspace_pages WHERE id = $1", &[&page_id])
                    .map_err(io::Error::other)?;
            }
        }
        refresh_positions(&mut transaction, workspace)?;
        write_collab(&mut transaction, collab)?;
        transaction.commit().map_err(io::Error::other)
    }

    fn persist_block(
        &self,
        workspace: &Workspace,
        collab: &CollabState,
        page_id: &str,
        block_id: &str,
    ) -> io::Result<()> {
        let mut client = self.lock();
        let mut transaction = client.transaction().map_err(io::Error::other)?;
        write_meta(&mut transaction, &WorkspaceMeta::from_workspace(workspace))?;
        let Some(position) = workspace.pages.iter().position(|page| page.id == page_id) else {
            transaction.commit().map_err(io::Error::other)?;
            return Ok(());
        };
        write_page(&mut transaction, &workspace.pages[position], position)?;
        match find_block_slot(&workspace.pages[position].blocks, block_id, None) {
            Some((parent_id, block_position, block)) => {
                write_block_row(
                    &mut transaction,
                    page_id,
                    parent_id.as_deref(),
                    block_position,
                    block,
                )?;
            }
            None => delete_block_subtree(&mut transaction, page_id, block_id)?,
        }
        write_collab(&mut transaction, collab)?;
        transaction.commit().map_err(io::Error::other)
    }
}

impl AuthStore for PostgresStore {
    fn find_account(&self, email: &str) -> io::Result<Option<Account>> {
        let mut client = self.lock();
        let row = client
            .query_opt(
                "SELECT id, member_id, email, password_hash, created_at
                 FROM accounts WHERE email = $1",
                &[&email],
            )
            .map_err(io::Error::other)?;
        Ok(row.map(|row| Account {
            id: row.get(0),
            member_id: row.get(1),
            email: row.get(2),
            password_hash: row.get(3),
            created_at: row.get(4),
        }))
    }

    fn upsert_account(&self, account: &Account) -> io::Result<()> {
        let mut client = self.lock();
        client
            .execute(
                "INSERT INTO accounts (id, member_id, email, password_hash, created_at)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (id) DO UPDATE SET
                     member_id = EXCLUDED.member_id,
                     email = EXCLUDED.email,
                     password_hash = EXCLUDED.password_hash,
                     created_at = EXCLUDED.created_at",
                &[
                    &account.id,
                    &account.member_id,
                    &account.email,
                    &account.password_hash,
                    &account.created_at,
                ],
            )
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn save_session(&self, session: &Session) -> io::Result<()> {
        let mut client = self.lock();
        client
            .execute(
                "INSERT INTO sessions (token_hash, member_id, created_at, expires_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (token_hash) DO UPDATE SET
                     member_id = EXCLUDED.member_id,
                     created_at = EXCLUDED.created_at,
                     expires_at = EXCLUDED.expires_at",
                &[
                    &session.token_hash,
                    &session.member_id,
                    &session.created_at,
                    &session.expires_at,
                ],
            )
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn find_session(&self, token_hash: &str) -> io::Result<Option<Session>> {
        let mut client = self.lock();
        let row = client
            .query_opt(
                "SELECT token_hash, member_id, created_at, expires_at
                 FROM sessions WHERE token_hash = $1",
                &[&token_hash],
            )
            .map_err(io::Error::other)?;
        Ok(row.map(|row| Session {
            token_hash: row.get(0),
            member_id: row.get(1),
            created_at: row.get(2),
            expires_at: row.get(3),
        }))
    }

    fn delete_session(&self, token_hash: &str) -> io::Result<()> {
        let mut client = self.lock();
        client
            .execute("DELETE FROM sessions WHERE token_hash = $1", &[&token_hash])
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn prune_sessions(&self, now: &str) -> io::Result<()> {
        let mut client = self.lock();
        client
            .execute("DELETE FROM sessions WHERE expires_at < $1", &[&now])
            .map_err(io::Error::other)?;
        Ok(())
    }
}

fn write_meta(transaction: &mut Transaction<'_>, meta: &WorkspaceMeta) -> io::Result<()> {
    let json = serde_json::to_string(meta).map_err(io::Error::other)?;
    transaction
        .execute(
            "INSERT INTO workspace_meta (id, json, updated_at) VALUES (1, $1, $2)
             ON CONFLICT (id) DO UPDATE SET json = EXCLUDED.json, updated_at = EXCLUDED.updated_at",
            &[&json, &now_iso()],
        )
        .map_err(io::Error::other)?;
    Ok(())
}

fn write_page(transaction: &mut Transaction<'_>, page: &Page, position: usize) -> io::Result<()> {
    let mut stored = page.clone();
    stored.blocks.clear();
    let json = serde_json::to_string(&stored).map_err(io::Error::other)?;
    transaction
        .execute(
            "INSERT INTO workspace_pages (id, position, json, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE SET
                 position = EXCLUDED.position,
                 json = EXCLUDED.json,
                 updated_at = EXCLUDED.updated_at",
            &[&page.id, &(position as i64), &json, &now_iso()],
        )
        .map_err(io::Error::other)?;
    Ok(())
}

fn write_all_pages(transaction: &mut Transaction<'_>, workspace: &Workspace) -> io::Result<()> {
    transaction
        .execute("DELETE FROM workspace_pages", &[])
        .map_err(io::Error::other)?;
    transaction
        .execute("DELETE FROM workspace_blocks", &[])
        .map_err(io::Error::other)?;
    for (position, page) in workspace.pages.iter().enumerate() {
        write_page(transaction, page, position)?;
        write_blocks_recursive(transaction, &page.id, None, &page.blocks)?;
    }
    Ok(())
}

fn refresh_positions(transaction: &mut Transaction<'_>, workspace: &Workspace) -> io::Result<()> {
    for (position, page) in workspace.pages.iter().enumerate() {
        transaction
            .execute(
                "UPDATE workspace_pages SET position = $1 WHERE id = $2",
                &[&(position as i64), &page.id],
            )
            .map_err(io::Error::other)?;
    }
    Ok(())
}

fn write_blocks_recursive(
    transaction: &mut Transaction<'_>,
    page_id: &str,
    parent_id: Option<&str>,
    blocks: &[Block],
) -> io::Result<()> {
    for (position, block) in blocks.iter().enumerate() {
        write_block_row(transaction, page_id, parent_id, position, block)?;
        write_blocks_recursive(
            transaction,
            page_id,
            Some(block.id.as_str()),
            &block.children,
        )?;
    }
    Ok(())
}

fn write_block_row(
    transaction: &mut Transaction<'_>,
    page_id: &str,
    parent_id: Option<&str>,
    position: usize,
    block: &Block,
) -> io::Result<()> {
    let mut stored = block.clone();
    stored.children.clear();
    let json = serde_json::to_string(&stored).map_err(io::Error::other)?;
    transaction
        .execute(
            "INSERT INTO workspace_blocks (id, page_id, parent_id, position, json, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE SET
                 page_id = EXCLUDED.page_id,
                 parent_id = EXCLUDED.parent_id,
                 position = EXCLUDED.position,
                 json = EXCLUDED.json,
                 updated_at = EXCLUDED.updated_at",
            &[
                &block.id,
                &page_id,
                &parent_id,
                &(position as i64),
                &json,
                &now_iso(),
            ],
        )
        .map_err(io::Error::other)?;
    Ok(())
}

fn write_collab(transaction: &mut Transaction<'_>, collab: &CollabState) -> io::Result<()> {
    let json = serde_json::to_string(collab).map_err(io::Error::other)?;
    transaction
        .execute(
            "INSERT INTO collab (id, json, updated_at) VALUES (1, $1, $2)
             ON CONFLICT (id) DO UPDATE SET json = EXCLUDED.json, updated_at = EXCLUDED.updated_at",
            &[&json, &now_iso()],
        )
        .map_err(io::Error::other)?;
    Ok(())
}

fn read_pages(client: &mut Client) -> io::Result<Vec<Page>> {
    let rows = client
        .query(
            "SELECT json FROM workspace_pages ORDER BY position, id",
            &[],
        )
        .map_err(io::Error::other)?;
    rows.into_iter()
        .map(|row| {
            let json: String = row.get(0);
            serde_json::from_str(&json)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .collect()
}

struct StoredBlock {
    parent_id: Option<String>,
    position: i64,
    block: Block,
}

fn read_blocks(client: &mut Client) -> io::Result<HashMap<String, Vec<StoredBlock>>> {
    let rows = client
        .query(
            "SELECT page_id, parent_id, position, json FROM workspace_blocks
             ORDER BY page_id, position, id",
            &[],
        )
        .map_err(io::Error::other)?;
    let mut by_page: HashMap<String, Vec<StoredBlock>> = HashMap::new();
    for row in rows {
        let page_id: String = row.get(0);
        let parent_id: Option<String> = row.get(1);
        let position: i64 = row.get(2);
        let json: String = row.get(3);
        let block: Block = serde_json::from_str(&json)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        by_page.entry(page_id).or_default().push(StoredBlock {
            parent_id,
            position,
            block,
        });
    }
    Ok(by_page)
}

fn attach_blocks(page: &mut Page, rows: Vec<StoredBlock>) {
    let mut children: HashMap<Option<String>, Vec<(i64, Block)>> = HashMap::new();
    for row in rows {
        children
            .entry(row.parent_id)
            .or_default()
            .push((row.position, row.block));
    }
    let mut visited = HashSet::new();
    page.blocks = build_level(None, &mut children, &mut visited);
}

fn build_level(
    parent: Option<&str>,
    children: &mut HashMap<Option<String>, Vec<(i64, Block)>>,
    visited: &mut HashSet<String>,
) -> Vec<Block> {
    let mut level = children
        .remove(&parent.map(str::to_string))
        .unwrap_or_default();
    level.sort_by_key(|(position, _)| *position);
    level
        .into_iter()
        .filter_map(|(_, mut block)| {
            if !visited.insert(block.id.clone()) {
                return None;
            }
            block.children = build_level(Some(&block.id), children, visited);
            Some(block)
        })
        .collect()
}

fn find_block_slot<'a>(
    blocks: &'a [Block],
    id: &str,
    parent_id: Option<&str>,
) -> Option<(Option<String>, usize, &'a Block)> {
    for (position, block) in blocks.iter().enumerate() {
        if block.id == id {
            return Some((parent_id.map(str::to_string), position, block));
        }
        if let Some(found) = find_block_slot(&block.children, id, Some(block.id.as_str())) {
            return Some(found);
        }
    }
    None
}

fn delete_block_subtree(
    transaction: &mut Transaction<'_>,
    page_id: &str,
    block_id: &str,
) -> io::Result<()> {
    let mut stack = vec![block_id.to_string()];
    while let Some(id) = stack.pop() {
        let rows = transaction
            .query(
                "SELECT id FROM workspace_blocks WHERE page_id = $1 AND parent_id = $2",
                &[&page_id, &id],
            )
            .map_err(io::Error::other)?;
        stack.extend(rows.into_iter().map(|row| row.get::<_, String>(0)));
        transaction
            .execute(
                "DELETE FROM workspace_blocks WHERE page_id = $1 AND id = $2",
                &[&page_id, &id],
            )
            .map_err(io::Error::other)?;
    }
    Ok(())
}
