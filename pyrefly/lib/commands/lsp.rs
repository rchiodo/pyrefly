/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashMap;
use std::collections::HashSet;
use std::num::NonZero;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use pyrefly_util::lock::Mutex;
use pyrefly_util::lock::RwLock;

use clap::Parser;
use clap::ValueEnum;
use crossbeam_channel::Select;
use crossbeam_channel::Sender;
use dupe::Dupe;
use itertools::__std_iter::once;
use itertools::Itertools;
use lsp_server::Connection;
use lsp_server::ErrorCode;
use lsp_server::Message;
use lsp_server::Notification;
use lsp_server::ProtocolError;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_server::ResponseError;
use lsp_types::CodeAction;
use lsp_types::CodeActionKind;
use lsp_types::CodeActionOptions;
use lsp_types::CodeActionOrCommand;
use lsp_types::CodeActionParams;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CodeActionResponse;
use lsp_types::CompletionList;
use lsp_types::CompletionOptions;
use lsp_types::CompletionParams;
use lsp_types::CompletionResponse;
use lsp_types::ConfigurationItem;
use lsp_types::ConfigurationParams;
use lsp_types::Diagnostic;
use lsp_types::DidChangeConfigurationParams;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidChangeWatchedFilesClientCapabilities;
use lsp_types::DidChangeWatchedFilesParams;
use lsp_types::DidChangeWatchedFilesRegistrationOptions;
use lsp_types::DidChangeWorkspaceFoldersParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::DidSaveTextDocumentParams;
use lsp_types::DocumentDiagnosticParams;
use lsp_types::DocumentDiagnosticReport;
use lsp_types::DocumentHighlight;
use lsp_types::DocumentHighlightParams;
use lsp_types::DocumentSymbol;
use lsp_types::DocumentSymbolParams;
use lsp_types::DocumentSymbolResponse;
use lsp_types::FileSystemWatcher;
use lsp_types::FullDocumentDiagnosticReport;
use lsp_types::GlobPattern;
use lsp_types::GotoDefinitionParams;
use lsp_types::GotoDefinitionResponse;
use lsp_types::Hover;
use lsp_types::HoverContents;
use lsp_types::HoverParams;
use lsp_types::HoverProviderCapability;
use lsp_types::InitializeParams;
use lsp_types::InlayHint;
use lsp_types::InlayHintLabel;
use lsp_types::InlayHintParams;
use lsp_types::Location;
use lsp_types::MarkupContent;
use lsp_types::MarkupKind;
use lsp_types::NumberOrString;
use lsp_types::OneOf;
use lsp_types::Position;
use lsp_types::PositionEncodingKind;
use lsp_types::PrepareRenameResponse;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::Range;
use lsp_types::ReferenceParams;
use lsp_types::Registration;
use lsp_types::RegistrationParams;
use lsp_types::RelatedFullDocumentDiagnosticReport;
use lsp_types::RelativePattern;
use lsp_types::RenameOptions;
use lsp_types::RenameParams;
use lsp_types::SemanticTokens;
use lsp_types::SemanticTokensFullOptions;
use lsp_types::SemanticTokensOptions;
use lsp_types::SemanticTokensParams;
use lsp_types::SemanticTokensRangeParams;
use lsp_types::SemanticTokensRangeResult;
use lsp_types::SemanticTokensResult;
use lsp_types::SemanticTokensServerCapabilities;
use lsp_types::ServerCapabilities;
use lsp_types::SignatureHelp;
use lsp_types::SignatureHelpOptions;
use lsp_types::SignatureHelpParams;
use lsp_types::SymbolInformation;
use lsp_types::TextDocumentContentChangeEvent;
use lsp_types::TextDocumentPositionParams;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::TextEdit;
use lsp_types::Unregistration;
use lsp_types::UnregistrationParams;
use lsp_types::Url;
use lsp_types::VersionedTextDocumentIdentifier;
use lsp_types::WatchKind;
use lsp_types::WorkspaceClientCapabilities;
use lsp_types::WorkspaceEdit;
use lsp_types::WorkspaceFoldersServerCapabilities;
use lsp_types::WorkspaceServerCapabilities;
use lsp_types::WorkspaceSymbolResponse;
use lsp_types::notification::Cancel;
use lsp_types::notification::DidChangeConfiguration;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidChangeWatchedFiles;
use lsp_types::notification::DidChangeWorkspaceFolders;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::DidSaveTextDocument;
use lsp_types::notification::Exit;
use lsp_types::notification::Notification as _;
use lsp_types::notification::PublishDiagnostics;
use lsp_types::request::CodeActionRequest;
use lsp_types::request::Completion;
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::request::DocumentHighlightRequest;
use lsp_types::request::DocumentSymbolRequest;
use lsp_types::request::GotoDefinition;
use lsp_types::request::HoverRequest;
use lsp_types::request::InlayHintRequest;
use lsp_types::request::PrepareRenameRequest;
use lsp_types::request::References;
use lsp_types::request::RegisterCapability;
use lsp_types::request::Rename;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::request::SemanticTokensRangeRequest;
use lsp_types::request::SignatureHelpRequest;
use lsp_types::request::UnregisterCapability;
use lsp_types::request::WorkspaceConfiguration;
use lsp_types::request::WorkspaceSymbolRequest;
use path_absolutize::Absolutize;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::module_path::ModulePath;
use ruff_python_ast::name::Name;
use crate::error::collector::ErrorCollector;
use crate::error::style::ErrorStyle;
use pyrefly_python::module_path::ModulePathDetails;
use pyrefly_util::arc_id::ArcId;
use pyrefly_util::arc_id::WeakArcId;
use pyrefly_util::args::clap_env;
use pyrefly_util::events::CategorizedEvents;

use pyrefly_util::prelude::VecExt;
use pyrefly_util::task_heap::CancellationHandle;
use pyrefly_util::task_heap::Cancelled;
use pyrefly_util::thread_pool::ThreadCount;
use pyrefly_util::thread_pool::ThreadPool;
use ruff_source_file::LineIndex;
use ruff_source_file::OneIndexed;
use ruff_source_file::SourceLocation;
use ruff_text_size::Ranged;
use ruff_text_size::TextRange;
use ruff_text_size::TextSize;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use starlark_map::small_map::SmallMap;
use starlark_map::small_set::SmallSet;
use tracing::error;
use tracing::warn;

use crate::commands::config_finder::standard_config_finder;
use crate::commands::run::CommandExitStatus;
use crate::commands::tsp;
use crate::commands::tsp::GetPythonSearchPathsRequest;
use crate::commands::tsp::GetSnapshotRequest;
use crate::commands::tsp::GetSymbolRequest;
use crate::commands::tsp::GetTypeRequest;
use crate::commands::tsp::ResolveImportDeclarationRequest;
use crate::commands::tsp::GetTypeOfDeclarationRequest;
use crate::commands::tsp::GetReprRequest;
use crate::commands::tsp::GetDocstringRequest;
use crate::commands::tsp::SearchForTypeAttributeRequest;
use crate::commands::util::module_from_path;
use crate::common::files::PYTHON_FILE_SUFFIXES_TO_WATCH;
use crate::config::config::ConfigFile;
use crate::config::config::ConfigSource;
use crate::config::environment::environment::PythonEnvironment;
use crate::config::finder::ConfigFinder;
use crate::config::util::ConfigOrigin;
use crate::error::error::Error;
use crate::error::kind::Severity;
use crate::module::bundled::typeshed;
use crate::module::module_info::ModuleInfo;
use crate::module::module_info::TextRangeWithModuleInfo;
use crate::state::handle::Handle;
use crate::state::lsp::FindDefinitionItem;
use crate::state::require::Require;
use crate::state::semantic_tokens::SemanticTokensLegends;
use crate::state::state::CommittingTransaction;
use crate::state::state::State;
use crate::state::state::Transaction;
use crate::state::state::TransactionData;
use crate::types::display::TypeDisplayContext;

/// Pyrefly's indexing strategy for open projects when performing go-to-definition
/// requests.
#[deny(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub(crate) enum IndexingMode {
    /// Do not index anything. Features that depend on indexing (e.g. find-refs) will be disabled.
    None,
    /// Start indexing when opening a file that belongs to a config in the background.
    /// Indexing will happen in another thread, so that normal IDE services are not blocked.
    #[default]
    LazyNonBlockingBackground,
    /// Start indexing when opening a file that belongs to a config in the background.
    /// Indexing will happen in the main thread, so that IDE services will be blocked.
    /// However, this is useful for deterministic testing.
    LazyBlocking,
}

/// Arguments for LSP server
#[deny(clippy::missing_docs_in_private_items)]
#[derive(Debug, Parser, Clone)]
pub struct Args {
    /// Find the struct that contains this field and add the indexing mode used by the language server
    #[arg(long, value_enum, default_value_t, env = clap_env("INDEXING_MODE"))]
    pub(crate) indexing_mode: IndexingMode,
}

/// `IDETransactionManager` aims to always produce a transaction that contains the up-to-date
/// in-memory contents.
#[derive(Default)]
struct IDETransactionManager<'a> {
    /// Invariant:
    /// If it's None, then the main `State` already contains up-to-date checked content
    /// of all in-memory files.
    /// Otherwise, it will contains up-to-date checked content of all in-memory files.
    saved_state: Option<TransactionData<'a>>,
}

impl<'a> IDETransactionManager<'a> {
    #[expect(clippy::result_large_err)] // Both results are basically the same size
    /// Produce a possibly committable transaction in order to recheck in-memory files.
    fn get_possibly_committable_transaction(
        &mut self,
        state: &'a State,
    ) -> Result<CommittingTransaction<'a>, Transaction<'a>> {
        // If there is no ongoing recheck due to on-disk changes, we should prefer to commit
        // the in-memory changes into the main state.
        if let Some(transaction) = state.try_new_committable_transaction(Require::Indexing, None) {
            // If we can commit in-memory changes, then there is no point of holding the
            // non-commitable transaction with a possibly outdated view of the `ReadableState`
            // so we can destroy the saved state.
            self.saved_state = None;
            Ok(transaction)
        } else {
            // If there is an ongoing recheck, trying to get a committable transaction will block
            // until the recheck is finished. This is bad for perceived perf. Therefore, we will
            // temporarily use a non-commitable transaction to hold the information that's necessary
            // to power IDE services.
            Err(self.non_commitable_transaction(state))
        }
    }

    /// Produce a `Transaction` to power readonly IDE services.
    /// This transaction will never be able to be committed.
    /// After using it, the state should be saved by calling the `save` method.
    ///
    /// The `Transaction` will always contain the handles of all open files with the latest content.
    /// It might be created fresh from state, or reused from previously saved state.
    fn non_commitable_transaction(&mut self, state: &'a State) -> Transaction<'a> {
        if let Some(saved_state) = self.saved_state.take() {
            saved_state.into_transaction()
        } else {
            state.transaction()
        }
    }

    /// This function should be called once we finished using transaction for an LSP request.
    fn save(&mut self, transaction: Transaction<'a>) {
        self.saved_state = Some(transaction.into_data())
    }
}

enum ServerEvent {
    // Part 1: Events that the server should try to handle first.
    /// Notify the server that recheck finishes, so server can revalidate all in-memory content
    /// based on the latest `State`.
    RecheckFinished,
    /// Inform the server that a request is cancelled.
    /// Server should know about this ASAP to avoid wasting time on cancelled requests.
    CancelRequest(RequestId),
    // Part 2: Events that can be queued in FIFO order and handled at a later time.
    DidOpenTextDocument(DidOpenTextDocumentParams),
    DidChangeTextDocument(DidChangeTextDocumentParams),
    DidCloseTextDocument(DidCloseTextDocumentParams),
    DidSaveTextDocument(DidSaveTextDocumentParams),
    DidChangeWatchedFiles(DidChangeWatchedFilesParams),
    DidChangeWorkspaceFolders(DidChangeWorkspaceFoldersParams),
    DidChangeConfiguration(DidChangeConfigurationParams),
    LspResponse(Response),
    LspRequest(Request),
    Exit,
}

#[derive(Clone, Dupe)]
struct ServerConnection(Arc<Connection>);

impl ServerConnection {
    fn send(&self, msg: Message) {
        if self.0.sender.send(msg).is_err() {
            // On error, we know the channel is closed.
            // https://docs.rs/crossbeam/latest/crossbeam/channel/struct.Sender.html#method.send
            eprintln!("Connection closed.");
        };
    }

    fn publish_diagnostics_for_uri(&self, uri: Url, diags: Vec<Diagnostic>, version: Option<i32>) {
        self.send(Message::Notification(
            new_notification::<PublishDiagnostics>(PublishDiagnosticsParams::new(
                uri, diags, version,
            )),
        ));
    }

    fn publish_diagnostics(&self, diags: SmallMap<PathBuf, Vec<Diagnostic>>) {
        for (path, diags) in diags {
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            match Url::from_file_path(&path) {
                Ok(uri) => self.publish_diagnostics_for_uri(uri, diags, None),
                Err(_) => eprint!("Unable to convert path to uri: {path:?}"),
            }
        }
    }
}

struct Server {
    connection: ServerConnection,
    /// A thread pool of size one for heavy read operations on the State
    async_state_read_threads: ThreadPool,
    priority_events_sender: Arc<Sender<ServerEvent>>,
    initialize_params: InitializeParams,
    indexing_mode: IndexingMode,
    state: Arc<State>,
    open_files: Arc<RwLock<HashMap<PathBuf, Arc<String>>>>,
    /// A set of configs where we have already indexed all the files within the config.
    indexed_configs: Mutex<HashSet<ArcId<ConfigFile>>>,
    cancellation_handles: Arc<Mutex<HashMap<RequestId, CancellationHandle>>>,
    workspaces: Arc<Workspaces>,
    outgoing_request_id: Arc<AtomicI32>,
    outgoing_requests: Mutex<HashMap<RequestId, Request>>,
    filewatcher_registered: Arc<AtomicBool>,
    version_info: Mutex<HashMap<PathBuf, i32>>,
    /// Lookup table from TSP type handles to internal pyrefly types
    /// This is cleared whenever the snapshot increments
    type_handle_lookup: Arc<RwLock<(i32, HashMap<String, crate::types::types::Type>)>>,
}

/// Information about the Python environment p
#[derive(Debug, Clone)]
struct PythonInfo {
    /// The path to the interpreter used to query this `PythonInfo`'s [`PythonEnvironment`].
    interpreter: PathBuf,
    /// The [`PythonEnvironment`] values all [`ConfigFile`]s in a given workspace should
    /// use if no explicit [`ConfigFile::python_interpreter`] is provided, or any
    /// `PythonEnvironment` values in that `ConfigFile` are unfiled. If the `interpreter
    /// provided fails to execute or is invalid, this `PythonEnvironment` might instead
    /// be a system interpreter or [`PythonEnvironment::pyrefly_default()`].
    env: PythonEnvironment,
}

impl PythonInfo {
    fn new(interpreter: PathBuf) -> Self {
        let (env, query_error) = PythonEnvironment::get_interpreter_env(&interpreter);
        if let Some(error) = query_error {
            error!("{error}");
        }
        Self { interpreter, env }
    }
}

/// LSP workspace settings: this is all that is necessary to run an LSP at a given root.
#[derive(Debug, Clone)]
struct Workspace {
    #[expect(dead_code)]
    root: PathBuf,
    python_info: Option<PythonInfo>,
    search_path: Option<Vec<PathBuf>>,
    disable_language_services: bool,
    disable_type_errors: bool,
}

impl Workspace {
    fn new(workspace_root: &Path, python_info: Option<PythonInfo>) -> Self {
        Self {
            root: workspace_root.to_path_buf(),
            python_info,
            search_path: None,
            disable_language_services: false,
            disable_type_errors: false,
        }
    }

    fn new_with_default_env(workspace_root: &Path) -> Self {
        Self::new(workspace_root, None)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            root: PathBuf::from("/"),
            python_info: None,
            search_path: None,
            disable_language_services: Default::default(),
            disable_type_errors: false,
        }
    }
}

/// A cache of loaded configs from the LSP's [`standard_config_finder`]. These
/// values are [`WeakArcId`]s, so they will be dropped when no other references
/// point to them. We use this for determining the list of files we need to watch
/// when setting up the watcher.
struct WeakConfigCache(Mutex<HashSet<WeakArcId<ConfigFile>>>);

impl WeakConfigCache {
    fn new() -> Self {
        Self(Mutex::new(HashSet::new()))
    }

    fn insert(&self, config: WeakArcId<ConfigFile>) {
        self.0.lock().insert(config);
    }

    /// Purge any [`WeakArcId`]s that are [`WeakArcId::vacant`], and return
    /// the remaining configs, converted to [`ArcId`]s.
    fn clean_and_get_configs(&self) -> SmallSet<ArcId<ConfigFile>> {
        let mut configs = self.0.lock();
        let purged_config_count = configs.extract_if(|c| c.vacant()).count();
        if purged_config_count != 0 {
            eprintln!("Cleared {purged_config_count} dropped configs from config cache");
        }
        SmallSet::from_iter(configs.iter().filter_map(|c| c.upgrade()))
    }

    /// Given an [`ArcId<ConfigFile>`], get glob patterns that should be watched by a file watcher.
    /// We return a tuple of root (non-pattern part of the path) and a pattern.
    fn get_loaded_config_paths_to_watch(config: ArcId<ConfigFile>) -> Vec<(PathBuf, String)> {
        let mut result = Vec::new();
        let config_root = if let ConfigSource::File(config_path) = &config.source
            && let Some(root) = config_path.parent()
        {
            Some(root)
        } else {
            config.source.root()
        };
        if let Some(config_root) = config_root {
            ConfigFile::CONFIG_FILE_NAMES.iter().for_each(|config| {
                result.push((config_root.to_path_buf(), format!("**/{config}")));
            });
        }
        config
            .search_path()
            .chain(config.site_package_path())
            .cartesian_product(PYTHON_FILE_SUFFIXES_TO_WATCH)
            .for_each(|(s, suffix)| {
                result.push((s.to_owned(), format!("**/*.{suffix}")));
            });
        result
    }

    /// Get glob patterns to watch all Python files under [`ConfigFile::search_path`] and
    /// [`ConfigFile::site_package_path`], clearing out any removed [`ConfigFile`]s as we go.
    fn get_patterns_for_cached_configs(&self) -> Vec<(PathBuf, String)> {
        self.clean_and_get_configs()
            .into_iter()
            .flat_map(Self::get_loaded_config_paths_to_watch)
            .collect::<Vec<_>>()
    }
}

struct Workspaces {
    /// If a workspace is not found, this one is used. It contains every possible file on the system but is lowest priority.
    default: RwLock<Workspace>,
    workspaces: RwLock<SmallMap<PathBuf, Workspace>>,
    loaded_configs: Arc<WeakConfigCache>,
}

impl Workspaces {
    fn new(default: Workspace) -> Self {
        Self {
            default: RwLock::new(default),
            workspaces: RwLock::new(SmallMap::new()),
            loaded_configs: Arc::new(WeakConfigCache::new()),
        }
    }

    fn get_with<F, R>(&self, uri: PathBuf, f: F) -> R
    where
        F: FnOnce(&Workspace) -> R,
    {
        let workspaces = self.workspaces.read();
        let default_workspace = self.default.read();
        let workspace = workspaces
            .iter()
            .filter(|(key, _)| uri.starts_with(key))
            .max_by(|(key1, _), (key2, _)| key2.ancestors().count().cmp(&key1.ancestors().count()))
            .map(|(_, workspace)| workspace);
        f(workspace.unwrap_or(&default_workspace))
    }

    fn config_finder(
        workspaces: &Arc<Workspaces>,
        loaded_configs: Arc<WeakConfigCache>,
    ) -> ConfigFinder {
        let workspaces = workspaces.dupe();
        standard_config_finder(Arc::new(move |dir, mut config| {
            if let Some(dir) = dir
                && config.interpreters.python_interpreter.is_none()
                && config.interpreters.conda_environment.is_none()
            {
                workspaces.get_with(dir.to_owned(), |w| {
                    if let Some(search_path) = w.search_path.clone() {
                        config.search_path_from_args = search_path;
                    }
                    if let Some(PythonInfo {
                        interpreter,
                        mut env,
                    }) = w.python_info.clone()
                    {
                        let site_package_path = config.python_environment.site_package_path.take();
                        env.site_package_path = site_package_path;
                        config.interpreters.python_interpreter =
                            Some(ConfigOrigin::auto(interpreter));
                        config.python_environment = env;
                    }
                })
            };

            // we print the errors here instead of returning them since
            // it gives the most immediate feedback for config loading errors
            for error in config.configure() {
                error!("Error configuring `ConfigFile`: {}", error.get_message());
            }
            let config = ArcId::new(config);

            loaded_configs.insert(config.downgrade());

            (config, Vec::new())
        }))
    }
}

/// At the time when we are ready to handle a new LSP event, it will help if we know the a list of
/// buffered requests and notifications ready to be processed, because we can potentially make smart
/// decisions (e.g. not process cancelled requests).
///
/// This function listens to the LSP events in the order they arrive, and dispatch them into event
/// channels with various priority:
/// - priority_events includes those that should be handled as soon as possible (e.g. know that a
///   request is cancelled)
/// - queued_events includes most of the other events.
fn dispatch_lsp_events(
    connection: &Connection,
    priority_events_sender: Arc<Sender<ServerEvent>>,
    queued_events_sender: Sender<ServerEvent>,
) {
    for msg in &connection.receiver {
        match msg {
            Message::Request(x) => {
                match connection.handle_shutdown(&x) {
                    Ok(is_shutdown) => {
                        if is_shutdown {
                            return;
                        }
                    }
                    Err(_) => {
                        return;
                    }
                }
                if queued_events_sender
                    .send(ServerEvent::LspRequest(x))
                    .is_err()
                {
                    return;
                }
            }
            Message::Response(x) => {
                if queued_events_sender
                    .send(ServerEvent::LspResponse(x))
                    .is_err()
                {
                    return;
                }
            }
            Message::Notification(x) => {
                let send_result = if let Some(params) = as_notification::<DidOpenTextDocument>(&x) {
                    queued_events_sender.send(ServerEvent::DidOpenTextDocument(params))
                } else if let Some(params) = as_notification::<DidChangeTextDocument>(&x) {
                    queued_events_sender.send(ServerEvent::DidChangeTextDocument(params))
                } else if let Some(params) = as_notification::<DidCloseTextDocument>(&x) {
                    queued_events_sender.send(ServerEvent::DidCloseTextDocument(params))
                } else if let Some(params) = as_notification::<DidSaveTextDocument>(&x) {
                    queued_events_sender.send(ServerEvent::DidSaveTextDocument(params))
                } else if let Some(params) = as_notification::<DidChangeWatchedFiles>(&x) {
                    queued_events_sender.send(ServerEvent::DidChangeWatchedFiles(params))
                } else if let Some(params) = as_notification::<DidChangeWorkspaceFolders>(&x) {
                    queued_events_sender.send(ServerEvent::DidChangeWorkspaceFolders(params))
                } else if let Some(params) = as_notification::<DidChangeConfiguration>(&x) {
                    queued_events_sender.send(ServerEvent::DidChangeConfiguration(params))
                } else if let Some(params) = as_notification::<Cancel>(&x) {
                    let id = match params.id {
                        NumberOrString::Number(i) => RequestId::from(i),
                        NumberOrString::String(s) => RequestId::from(s),
                    };
                    priority_events_sender.send(ServerEvent::CancelRequest(id))
                } else if as_notification::<Exit>(&x).is_some() {
                    queued_events_sender.send(ServerEvent::Exit)
                } else {
                    eprintln!("Unhandled notification: {x:?}");
                    Ok(())
                };
                if send_result.is_err() {
                    return;
                }
            }
        }
    }
}

fn initialize_connection(
    connection: &Connection,
    args: &Args,
) -> Result<InitializeParams, ProtocolError> {
    let (request_id, initialization_params) = connection.initialize_start()?;
    let initialization_params: InitializeParams =
        serde_json::from_value(initialization_params).unwrap();
    let augments_syntax_tokens = initialization_params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|c| c.semantic_tokens.as_ref())
        .and_then(|c| c.augments_syntax_tokens)
        .unwrap_or(false);
    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
            ..Default::default()
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_owned()]),
            ..Default::default()
        }),
        document_highlight_provider: Some(OneOf::Left(true)),
        // Find references won't work properly if we don't know all the files.
        references_provider: match args.indexing_mode {
            IndexingMode::None => None,
            IndexingMode::LazyNonBlockingBackground | IndexingMode::LazyBlocking => {
                Some(OneOf::Left(true))
            }
        },
        rename_provider: match args.indexing_mode {
            IndexingMode::None => None,
            IndexingMode::LazyNonBlockingBackground | IndexingMode::LazyBlocking => {
                Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                }))
            }
        },
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_owned(), ",".to_owned()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: if augments_syntax_tokens {
            // We currently only return partial tokens (e.g. no tokens for keywords right now).
            // If the client doesn't support `augments_syntax_tokens` to fallback baseline
            // syntax highlighting for tokens we don't provide, it will be a regression
            // (e.g. users might lose keyword highlighting).
            // Therefore, we should not produce semantic tokens if the client doesn't support `augments_syntax_tokens`.
            Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                SemanticTokensOptions {
                    legend: SemanticTokensLegends::lsp_semantic_token_legends(),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    range: Some(true),
                    ..Default::default()
                },
            ))
        } else {
            None
        },
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(OneOf::Left(true)),
            }),
            file_operations: None,
        }),
        ..Default::default()
    })
    .unwrap();
    let initialize_data = serde_json::json!({
        "capabilities": server_capabilities,
    });

    connection.initialize_finish(request_id, initialize_data)?;
    Ok(initialization_params)
}

pub fn run_lsp(
    connection: Arc<Connection>,
    wait_on_connection: impl FnOnce() -> anyhow::Result<()> + Send + 'static,
    args: Args,
) -> anyhow::Result<CommandExitStatus> {
    let initialization_params = match initialize_connection(&connection, &args) {
        Ok(it) => it,
        Err(e) => {
            // Use this in later versions of LSP server
            // if e.channel_is_disconnected() {
            // io_threads.join()?;
            // }
            return Err(e.into());
        }
    };
    eprintln!("Reading messages");
    let connection_for_dispatcher = connection.dupe();
    let (queued_events_sender, queued_events_receiver) = crossbeam_channel::unbounded();
    let (priority_events_sender, priority_events_receiver) = crossbeam_channel::unbounded();
    let priority_events_sender = Arc::new(priority_events_sender);
    let mut event_receiver_selector = Select::new_biased();
    // Biased selector will pick the receiver with lower index over higher ones,
    // so we register priority_events_receiver first.
    let priority_receiver_index = event_receiver_selector.recv(&priority_events_receiver);
    let queued_events_receiver_index = event_receiver_selector.recv(&queued_events_receiver);
    let server = Server::new(
        connection,
        priority_events_sender.dupe(),
        initialization_params,
        args.indexing_mode,
    );
    std::thread::spawn(move || {
        dispatch_lsp_events(
            &connection_for_dispatcher,
            priority_events_sender,
            queued_events_sender,
        );
    });
    let mut ide_transaction_manager = IDETransactionManager::default();
    let mut canceled_requests = HashSet::new();
    loop {
        let selected = event_receiver_selector.select();
        let received = match selected.index() {
            i if i == priority_receiver_index => selected.recv(&priority_events_receiver),
            i if i == queued_events_receiver_index => selected.recv(&queued_events_receiver),
            _ => unreachable!(),
        };
        if let Ok(event) = received {
            match server.process_event(
                &mut ide_transaction_manager,
                &mut canceled_requests,
                event,
            )? {
                ProcessEvent::Continue => {}
                ProcessEvent::Exit => break,
            }
        } else {
            break;
        }
    }
    eprintln!("waiting for connection to close");
    drop(server); // close connection
    wait_on_connection()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(CommandExitStatus::Success)
}

impl Args {
    pub fn run(self) -> anyhow::Result<CommandExitStatus> {
        // Note that  we must have our logging only write out to stderr.
        eprintln!("starting generic LSP server");

        // Create the transport. Includes the stdio (stdin and stdout) versions but this could
        // also be implemented to use sockets or HTTP.
        let (connection, io_threads) = Connection::stdio();

        run_lsp(
            Arc::new(connection),
            move || io_threads.join().map_err(anyhow::Error::from),
            self,
        )
    }
}

/// Convert to a path we can show to the user. The contents may not match the disk, but it has
/// to be basically right.
fn to_real_path(path: &ModulePath) -> Option<PathBuf> {
    match path.details() {
        ModulePathDetails::FileSystem(path)
        | ModulePathDetails::Memory(path)
        | ModulePathDetails::Namespace(path) => Some(path.to_path_buf()),
        ModulePathDetails::BundledTypeshed(path) => {
            let typeshed = typeshed().ok()?;
            let typeshed_path = match typeshed.materialized_path_on_disk() {
                Ok(typeshed_path) => Some(typeshed_path),
                Err(err) => {
                    warn!("Builtins unable to be loaded on disk, {}", err);
                    None
                }
            }?;
            Some(typeshed_path.join(path))
        }
    }
}

fn module_info_to_uri(module_info: &ModuleInfo) -> Option<Url> {
    let path = to_real_path(module_info.path())?;
    let abs_path = path.absolutize();
    let abs_path = abs_path.as_deref().unwrap_or(&path);
    Some(Url::from_file_path(abs_path).unwrap())
}

enum ProcessEvent {
    Continue,
    Exit,
}

/// Format a TSP Type into a human-readable string representation
fn format_type_representation(type_param: &tsp::Type, flags: tsp::TypeReprFlags) -> String {
    use tsp::TypeCategory;
    
    let mut result = String::new();
    
    // Handle different type categories
    match type_param.category {
        TypeCategory::ANY => result.push_str("Any"),
        TypeCategory::FUNCTION => {
            // For functions, show signature if available
            if type_param.name.is_empty() {
                result.push_str("Callable[..., Any]");
            } else {
                result.push_str(&type_param.name);
            }
        },
        TypeCategory::OVERLOADED => {
            result.push_str("Overload[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::CLASS => {
            // For classes, show the class name
            if flags.has_convert_to_instance_type() {
                // Convert to instance type representation
                result.push_str(&type_param.name);
            } else {
                // Show as type
                result.push_str("type[");
                result.push_str(&type_param.name);
                result.push(']');
            }
        },
        TypeCategory::MODULE => {
            result.push_str("Module[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::UNION => {
            // For unions, we'd need to format multiple types
            result.push_str("Union[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::TYPE_VAR => {
            result.push_str(&type_param.name);
            // Add variance information if requested
            if flags.has_print_type_var_variance() {
                // This would require additional metadata about variance
                // For now, just show the basic type var name
            }
        },
        _ => {
            // Default case for unknown categories
            if type_param.name.is_empty() {
                result.push_str("Unknown");
            } else {
                result.push_str(&type_param.name);
            }
        }
    }
    
    // Add module information if available and it's not a builtin
    if let Some(module_name) = &type_param.module_name {
        if !module_name.name_parts.is_empty() && module_name.name_parts[0] != "builtins" {
            let module_path = module_name.name_parts.join(".");
            result = format!("{}.{}", module_path, result);
        }
    }

    result
}
const PYTHON_SECTION: &str = "python";

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PyreflyClientConfig {
    disable_type_errors: Option<bool>,
    disable_language_services: Option<bool>,
    extra_paths: Option<Vec<PathBuf>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LspConfig {
    python_path: Option<String>,
    pyrefly: Option<PyreflyClientConfig>,
}

impl Server {
    const FILEWATCHER_ID: &str = "FILEWATCHER";

    /// Helper function to create a consistent "Snapshot is outdated" error response
    fn snapshot_outdated_error() -> ResponseError {
        ResponseError {
            code: ErrorCode::ServerCancelled as i32,
            message: "Snapshot is outdated".to_string(),
            data: None,
        }
    }

    /// Process the event and return next step.
    fn process_event<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        canceled_requests: &mut HashSet<RequestId>,
        event: ServerEvent,
    ) -> anyhow::Result<ProcessEvent> {
        match event {
            ServerEvent::Exit => {
                return Ok(ProcessEvent::Exit);
            }
            ServerEvent::RecheckFinished => {
                self.validate_in_memory(ide_transaction_manager)?;
            }
            ServerEvent::CancelRequest(id) => {
                eprintln!("We should cancel request {id:?}");
                if let Some(cancellation_handle) = self.cancellation_handles.lock().remove(&id) {
                    cancellation_handle.cancel();
                }
                canceled_requests.insert(id);
            }
            ServerEvent::DidOpenTextDocument(params) => {
                self.did_open(ide_transaction_manager, params)?;
            }
            ServerEvent::DidChangeTextDocument(params) => {
                self.did_change(ide_transaction_manager, params)?;
            }
            ServerEvent::DidCloseTextDocument(params) => {
                self.did_close(params)?;
            }
            ServerEvent::DidSaveTextDocument(params) => {
                self.did_save(params)?;
            }
            ServerEvent::DidChangeWatchedFiles(params) => {
                self.did_change_watched_files(params)?;
            }
            ServerEvent::DidChangeWorkspaceFolders(params) => {
                self.workspace_folders_changed(params);
            }
            ServerEvent::DidChangeConfiguration(params) => {
                self.did_change_configuration(ide_transaction_manager, params)?;
            }
            ServerEvent::LspResponse(x) => {
                if let Some(request) = self.outgoing_requests.lock().remove(&x.id) {
                    self.handle_response(ide_transaction_manager, &request, &x)?;
                } else {
                    eprintln!("Response for unknown request: {x:?}");
                }
            }
            ServerEvent::LspRequest(x) => {
                if canceled_requests.remove(&x.id) {
                    let message = format!("Request {} is canceled", x.id);
                    eprintln!("{message}");
                    self.send_response(Response::new_err(
                        x.id,
                        ErrorCode::RequestCanceled as i32,
                        message,
                    ));
                    return Ok(ProcessEvent::Continue);
                }
                eprintln!("Handling non-canceled request {} ({})", x.method, x.id);
                if let Some(params) = as_request::<GotoDefinition>(&x) {
                    let default_response = GotoDefinitionResponse::Array(Vec::new());
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self
                            .goto_definition(&transaction, params)
                            .unwrap_or(default_response)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<CodeActionRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.code_action(&transaction, params).unwrap_or_default()),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<Completion>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(x.id, self.completion(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<DocumentHighlightRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.document_highlight(&transaction, params)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<References>(&x) {
                    self.references(x.id, ide_transaction_manager, params);
                } else if let Some(params) = as_request::<PrepareRenameRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.prepare_rename(&transaction, params)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<Rename>(&x) {
                    self.rename(x.id, ide_transaction_manager, params);
                } else if let Some(params) = as_request::<SignatureHelpRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.signature_help(&transaction, params)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<HoverRequest>(&x) {
                    let default_response = Hover {
                        contents: HoverContents::Array(Vec::new()),
                        range: None,
                    };
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.hover(&transaction, params).unwrap_or(default_response)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<InlayHintRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.inlay_hints(&transaction, params).unwrap_or_default()),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<SemanticTokensFullRequest>(&x) {
                    let default_response = SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: Vec::new(),
                    });
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self
                            .semantic_tokens_full(&transaction, params)
                            .unwrap_or(default_response)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<SemanticTokensRangeRequest>(&x) {
                    let default_response = SemanticTokensRangeResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: Vec::new(),
                    });
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self
                            .semantic_tokens_ranged(&transaction, params)
                            .unwrap_or(default_response)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<DocumentSymbolRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(DocumentSymbolResponse::Nested(
                            self.hierarchical_document_symbols(&transaction, params)
                                .unwrap_or_default(),
                        )),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<WorkspaceSymbolRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(WorkspaceSymbolResponse::Flat(
                            self.workspace_symbols(&transaction, &params.query),
                        )),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<DocumentDiagnosticRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(
                        x.id,
                        Ok(self.document_diagnostics(&transaction, params)),
                    ));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<GetPythonSearchPathsRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response(x.id, Ok(self.get_python_search_paths(&transaction, params))));
                    ide_transaction_manager.save(transaction);
                } else if let Some(_params) = as_request::<GetSnapshotRequest>(&x) {
                    self.send_response(new_response(x.id, Ok(self.current_snapshot())));
                } else if let Some(params) = as_request::<GetTypeRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.get_type(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<GetSymbolRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.get_symbol(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<ResolveImportDeclarationRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.resolve_import_declaration(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<GetTypeOfDeclarationRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.get_type_of_declaration(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<GetReprRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.get_repr(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<GetDocstringRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.get_docstring(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else if let Some(params) = as_request::<SearchForTypeAttributeRequest>(&x) {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.state);
                    self.send_response(new_response_with_error_code(x.id, self.search_for_type_attribute(&transaction, params)));
                    ide_transaction_manager.save(transaction);
                } else {
                    eprintln!("Unhandled request: {x:?}");
                }
            }
        }
        Ok(ProcessEvent::Continue)
    }

    fn new(
        connection: Arc<Connection>,
        priority_events_sender: Arc<Sender<ServerEvent>>,
        initialize_params: InitializeParams,
        indexing_mode: IndexingMode,
    ) -> Self {
        let folders = if let Some(capability) = &initialize_params.capabilities.workspace
            && let Some(true) = capability.workspace_folders
            && let Some(folders) = &initialize_params.workspace_folders
        {
            folders
                .iter()
                .map(|x| x.uri.to_file_path().unwrap())
                .collect()
        } else {
            Vec::new()
        };

        let workspaces = Arc::new(Workspaces::new(Workspace::default()));

        let config_finder =
            Workspaces::config_finder(&workspaces, workspaces.loaded_configs.clone());
        let s = Self {
            connection: ServerConnection(connection),
            async_state_read_threads: ThreadPool::with_thread_count(ThreadCount::NumThreads(
                NonZero::new(1).unwrap(),
            )),
            priority_events_sender,
            initialize_params,
            indexing_mode,
            state: Arc::new(State::new(config_finder)),
            open_files: Arc::new(RwLock::new(HashMap::new())),
            indexed_configs: Mutex::new(HashSet::new()),
            cancellation_handles: Arc::new(Mutex::new(HashMap::new())),
            workspaces,
            outgoing_request_id: Arc::new(AtomicI32::new(1)),
            outgoing_requests: Mutex::new(HashMap::new()),
            filewatcher_registered: Arc::new(AtomicBool::new(false)),
            version_info: Mutex::new(HashMap::new()),
            type_handle_lookup: Arc::new(RwLock::new((0, HashMap::new()))),
        };
        s.configure(&folders, &[]);

        s
    }

    fn send_response(&self, x: Response) {
        self.connection.send(Message::Response(x))
    }

    fn send_request<T>(&self, params: T::Params)
    where
        T: lsp_types::request::Request,
    {
        let id = RequestId::from(self.outgoing_request_id.fetch_add(1, Ordering::SeqCst));
        let request = Request {
            id: id.clone(),
            method: T::METHOD.to_owned(),
            params: serde_json::to_value(params).unwrap(),
        };
        self.connection.send(Message::Request(request.clone()));
        self.outgoing_requests.lock().insert(id, request);
    }

    fn validate_in_memory_for_transaction(
        state: &State,
        open_files: &RwLock<HashMap<PathBuf, Arc<String>>>,
        transaction: &mut Transaction<'_>,
    ) -> Vec<(Handle, Require)> {
        let handles = open_files
            .read()
            .keys()
            .map(|x| (Self::make_open_handle(state, x), Require::Everything))
            .collect::<Vec<_>>();
        transaction.set_memory(
            open_files
                .read()
                .iter()
                .map(|x| (x.0.clone(), Some(x.1.dupe())))
                .collect::<Vec<_>>(),
        );
        transaction.run(&handles);
        handles
    }

    fn get_diag_if_shown(
        &self,
        e: &Error,
        open_files: &HashMap<PathBuf, Arc<String>>,
    ) -> Option<(PathBuf, Diagnostic)> {
        if let Some(path) = to_real_path(e.path()) {
            // When no file covers this, we'll get the default configured config which includes "everything"
            // and excludes `.<file>`s.
            let config = self
                .state
                .config_finder()
                .python_file(ModuleName::unknown(), e.path());
            if open_files.contains_key(&path)
                && !config.project_excludes.covers(&path)
                && !self
                    .workspaces
                    .get_with(path.to_path_buf(), |w| w.disable_type_errors)
            {
                return Some((
                    path.to_path_buf(),
                    Diagnostic {
                        range: e.lined_buffer().to_lsp_range(e.range()),
                        severity: Some(match e.severity() {
                            Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
                            Severity::Warn => lsp_types::DiagnosticSeverity::WARNING,
                            Severity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
                            // Ignored errors shouldn't be here
                            Severity::Ignore => lsp_types::DiagnosticSeverity::INFORMATION,
                        }),
                        source: Some("Pyrefly".to_owned()),
                        message: e.msg().to_owned(),
                        code: Some(lsp_types::NumberOrString::String(
                            e.error_kind().to_name().to_owned(),
                        )),
                        ..Default::default()
                    },
                ));
            }
        }
        None
    }

    fn validate_in_memory<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
    ) -> anyhow::Result<()> {
        let mut possibly_committable_transaction =
            ide_transaction_manager.get_possibly_committable_transaction(&self.state);
        let transaction = match &mut possibly_committable_transaction {
            Ok(transaction) => transaction.as_mut(),
            Err(transaction) => transaction,
        };
        let handles =
            Self::validate_in_memory_for_transaction(&self.state, &self.open_files, transaction);

        let publish = |transaction: &Transaction| {
            let mut diags: SmallMap<PathBuf, Vec<Diagnostic>> = SmallMap::new();
            let open_files = self.open_files.read();
            for x in open_files.keys() {
                diags.insert(x.as_path().to_owned(), Vec::new());
            }
            for e in transaction
                .get_errors(handles.iter().map(|(handle, _)| handle))
                .collect_errors()
                .shown
            {
                if let Some((path, diag)) = self.get_diag_if_shown(&e, &open_files) {
                    diags.entry(path.to_owned()).or_default().push(diag);
                }
            }
            self.connection.publish_diagnostics(diags);
        };

        match possibly_committable_transaction {
            Ok(transaction) => {
                self.state.commit_transaction(transaction);
                // In the case where we can commit transactions, `State` already has latest updates.
                // Therefore, we can compute errors from transactions freshly created from `State``.
                let transaction = self.state.transaction();
                publish(&transaction);
            }
            Err(transaction) => {
                // In the case where transaction cannot be committed because there is an ongoing
                // recheck, we still want to update the diagnostics. In this case, we compute them
                // from the transactions that won't be committed. It will still contain all the
                // up-to-date in-memory content, but can have stale main `State` content.
                publish(&transaction);
                ide_transaction_manager.save(transaction);
            }
        }

        Ok(())
    }

    fn populate_project_files_if_necessary(
        &self,
        config_to_populate_files: Option<ArcId<ConfigFile>>,
    ) {
        if let Some(config) = config_to_populate_files {
            match self.indexing_mode {
                IndexingMode::None => {}
                IndexingMode::LazyNonBlockingBackground => {
                    if self.indexed_configs.lock().insert(config.dupe()) {
                        let state = self.state.dupe();
                        let priority_events_sender = self.priority_events_sender.dupe();
                        std::thread::spawn(move || {
                            Self::populate_all_project_files_in_config(
                                config,
                                state,
                                priority_events_sender,
                            );
                        });
                    }
                }
                IndexingMode::LazyBlocking => {
                    if self.indexed_configs.lock().insert(config.dupe()) {
                        Self::populate_all_project_files_in_config(
                            config,
                            self.state.dupe(),
                            self.priority_events_sender.dupe(),
                        );
                    }
                }
            }
        }
    }

    /// Perform an invalidation of elements on `State` and commit them.
    /// Runs asynchronously. Returns immediately and may wait a while for a committable transaction.
    fn invalidate(&self, f: impl FnOnce(&mut Transaction) + Send + 'static) {
        let state = self.state.dupe();
        let priority_events_sender = self.priority_events_sender.dupe();
        let cancellation_handles = self.cancellation_handles.dupe();
        std::thread::spawn(move || {
            let mut transaction = state.new_committable_transaction(Require::Indexing, None);
            f(transaction.as_mut());
            // Commit will be blocked until there are no ongoing reads.
            // If we have some long running read jobs that can be cancelled, we should cancel them
            // to unblock committing transactions.
            for (_, cancellation_handle) in cancellation_handles.lock().drain() {
                cancellation_handle.cancel();
            }
            // we have to run, not just commit to process updates
            state.run_with_committing_transaction(transaction, &[]);
            state.increment_snapshot(); // Increment snapshot after invalidation
            // After we finished a recheck asynchronously, we immediately send `RecheckFinished` to
            // the main event loop of the server. As a result, the server can do a revalidation of
            // all the in-memory files based on the fresh main State as soon as possible.
            let _ = priority_events_sender.send(ServerEvent::RecheckFinished);
        });
    }

    fn validate_with_disk_invalidation(&self, invalidate_disk: Vec<PathBuf>) -> anyhow::Result<()> {
        if !invalidate_disk.is_empty() {
            self.invalidate(move |t| t.invalidate_disk(&invalidate_disk));
        }
        Ok(())
    }

    /// Certain IDE features (e.g. find-references) require us to know the dependency graph of the
    /// entire project to work. This blocking function should be called when we know that a project
    /// file is opened and if we intend to provide features like find-references, and should be
    /// called when config changes (currently this is a TODO).
    fn populate_all_project_files_in_config(
        config: ArcId<ConfigFile>,
        state: Arc<State>,
        priority_events_sender: Arc<Sender<ServerEvent>>,
    ) {
        let unknown = ModuleName::unknown();

        eprintln!("Populating all files in the config ({:?}).", config.root);
        let mut transaction = state.new_committable_transaction(Require::Indexing, None);

        let project_path_blobs = config.get_filtered_globs(None);
        let paths = project_path_blobs.files().unwrap_or_default();
        let mut handles = Vec::new();
        for path in paths {
            let module_path = ModulePath::filesystem(path.clone());
            let path_config = state.config_finder().python_file(unknown, &module_path);
            if config != path_config {
                continue;
            }
            let module_name = module_from_path(&path, path_config.search_path())
                .unwrap_or_else(ModuleName::unknown);
            handles.push((
                Handle::new(module_name, module_path, path_config.get_sys_info()),
                Require::Indexing,
            ));
        }

        eprintln!("Prepare to check {} files.", handles.len());
        transaction.as_mut().run(&handles);
        state.commit_transaction(transaction);
        // After we finished a recheck asynchronously, we immediately send `RecheckFinished` to
        // the main event loop of the server. As a result, the server can do a revalidation of
        // all the in-memory files based on the fresh main State as soon as possible.
        let _ = priority_events_sender.send(ServerEvent::RecheckFinished);
        eprintln!("Populated all files in the project path.");
    }

    fn did_save(&self, params: DidSaveTextDocumentParams) -> anyhow::Result<()> {
        let uri = params.text_document.uri.to_file_path().unwrap();
        self.validate_with_disk_invalidation(vec![uri])
    }

    fn did_open<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        params: DidOpenTextDocumentParams,
    ) -> anyhow::Result<()> {
        let uri = params.text_document.uri.to_file_path().unwrap();
        let config_to_populate_files = if self.indexing_mode != IndexingMode::None
            && let Some(directory) = uri.as_path().parent()
        {
            self.state.config_finder().directory(directory)
        } else {
            None
        };
        self.version_info
            .lock()
            .insert(uri.clone(), params.text_document.version);
        self.open_files
            .write()
            .insert(uri, Arc::new(params.text_document.text));
        self.state.increment_snapshot(); // Increment snapshot on file open
        self.validate_in_memory(ide_transaction_manager)?;
        self.populate_project_files_if_necessary(config_to_populate_files);
        // rewatch files in case we loaded or dropped any configs
        self.setup_file_watcher_if_necessary(
            self.workspaces
                .workspaces
                .read()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .as_slice(),
        );
        Ok(())
    }

    fn did_change<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        params: DidChangeTextDocumentParams,
    ) -> anyhow::Result<()> {
        /// Convert lsp_types::Position to usize index for a given text.
        fn position_to_usize(
            position: lsp_types::Position,
            index: &LineIndex,
            source_text: &str,
        ) -> usize {
            let source_location = SourceLocation {
                line: OneIndexed::from_zero_indexed(position.line as usize),
                character_offset: OneIndexed::from_zero_indexed(position.character as usize),
            };
            let text_size = index.offset(
                source_location,
                source_text,
                ruff_source_file::PositionEncoding::Utf16,
            );
            text_size.to_usize()
        }

        let VersionedTextDocumentIdentifier { uri, version } = params.text_document;
        let file_path = uri.to_file_path().unwrap();

        let mut version_info = self.version_info.lock();
        let old_version = version_info.get(&file_path).unwrap_or(&0);
        if version < *old_version {
            return Err(anyhow::anyhow!(
                "Unexpected version in didChange notification: {version:?} is less than {old_version:?}"
            ));
        }
        version_info.insert(file_path.clone(), version);
        let mut new_text = String::from(self.open_files.read().get(&file_path).unwrap().as_ref());
        for change in params.content_changes {
            let TextDocumentContentChangeEvent { range, text, .. } = change;
            // If no range is given, we can full text replace.
            let Some(range) = range else {
                new_text = text;
                continue;
            };
            let index = LineIndex::from_source_text(&new_text);
            let start = position_to_usize(range.start, &index, &new_text);
            let end = position_to_usize(range.end, &index, &new_text);
            new_text.replace_range(start..end, &text);
        }
        self.open_files
            .write()
            .insert(file_path.clone(), Arc::new(new_text));
        self.validate_in_memory(ide_transaction_manager)
    }

    fn get_type(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Convert Node to URI and position
        let uri = &params.node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_string(),
                data: None,
            });
        };

        // Convert offset to TextSize
        let position = TextSize::new(params.node.start as u32);

        // Get the type at the position
        let Some(type_info) = transaction.get_type_at(&handle, position) else {
            let short_uri = uri.to_string().chars().take(100).collect::<String>();
            eprintln!("Warning: No type found at position {} in {}", position.to_usize(), short_uri);
            return Ok(None);
        };

        // Convert pyrefly Type to TSP Type format
        Ok(Some(self.convert_and_register_type(type_info)))
    }

    fn get_symbol(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetSymbolParams,
    ) -> Result<Option<tsp::Symbol>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Convert Node to URI and position
        let uri = &params.node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_string(),
                data: None,
            });
        };

        // Get module info for position conversion
        // If the module is not loaded in the transaction, try to load it
        let (module_info, transaction_to_use) = match transaction.get_module_info(&handle) {
            Some(info) => (info, None), // Use the existing transaction
            None => {
                // Module not loaded in transaction, try to load it
                let Some(fresh_transaction) = self.load_module_if_needed(transaction, &handle) else {
                    return Err(ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: "Failed to load module".to_string(),
                        data: None,
                    });
                };
                
                let Some(info) = fresh_transaction.get_module_info(&handle) else {
                    return Err(ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: "Failed to get module info after loading".to_string(),
                        data: None,
                    });
                };
                
                (info, Some(fresh_transaction))
            }
        };

        // Use the appropriate transaction for the rest of the function
        let active_transaction = transaction_to_use.as_ref().unwrap_or(transaction);

        // Convert offset to TextSize
        let position = TextSize::new(params.node.start as u32);

        // First, check if we can get type information at this position
        let type_info = active_transaction.get_type_at(&handle, position);

        // Try to find definition at the position
        let (symbol_name, declarations, synthesized_types) = 
            if let Some(first_definition) = active_transaction.find_definition(&handle, position, true).first() {
                let definition_metadata = &first_definition.metadata;
                let definition = &first_definition.location;
                let _docstring = &first_definition.docstring;
                
                // Use provided name or extract from definition
                let name = params.name.unwrap_or_else(|| {
                    // Try to extract symbol name from the source code at the position
                    let start = position;
                    let end = TextSize::new((params.node.start + params.node.length) as u32);
                    module_info.code_at(TextRange::new(start, end)).to_string()
                });

                // Create declarations from the definition
                let mut decls = Vec::new();
                
                // Generate a unique handle for this declaration
                let declaration_handle = tsp::TypeHandle::String(format!("decl_{:p}_{}", &definition_metadata as *const _, u32::from(position)));
                
                // Determine the category and flags based on definition metadata
                let (category, flags) = match &definition_metadata {
                    crate::state::lsp::DefinitionMetadata::Variable(Some(symbol_kind)) => {
                        match symbol_kind {
                            pyrefly_python::symbol_kind::SymbolKind::Function => (tsp::DeclarationCategory::FUNCTION, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Class => (tsp::DeclarationCategory::CLASS, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Variable => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Constant => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_constant()),
                            pyrefly_python::symbol_kind::SymbolKind::Parameter => (tsp::DeclarationCategory::PARAM, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::TypeParameter => (tsp::DeclarationCategory::TYPE_PARAM, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::TypeAlias => (tsp::DeclarationCategory::TYPE_ALIAS, tsp::DeclarationFlags::new()),
                            _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                        }
                    },
                    crate::state::lsp::DefinitionMetadata::Module => {
                        // For module imports, check if type info is available to determine if resolved
                        let mut import_flags = tsp::DeclarationFlags::new();
                        if type_info.is_none() {
                            // If we can't get type info for an import, it might be unresolved
                            import_flags = import_flags.with_unresolved_import();
                        }
                        (tsp::DeclarationCategory::IMPORT, import_flags)
                    },
                    crate::state::lsp::DefinitionMetadata::Attribute(_) => {
                        // Attributes are typically class members
                        (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member())
                    },
                    crate::state::lsp::DefinitionMetadata::VariableOrAttribute(_, Some(symbol_kind)) => {
                        match symbol_kind {
                            pyrefly_python::symbol_kind::SymbolKind::Function => (tsp::DeclarationCategory::FUNCTION, tsp::DeclarationFlags::new().with_class_member()),
                            pyrefly_python::symbol_kind::SymbolKind::Class => (tsp::DeclarationCategory::CLASS, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Variable => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member()),
                            pyrefly_python::symbol_kind::SymbolKind::Constant => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member().with_constant()),
                            pyrefly_python::symbol_kind::SymbolKind::Attribute => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member()),
                            pyrefly_python::symbol_kind::SymbolKind::Parameter => (tsp::DeclarationCategory::PARAM, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::TypeParameter => (tsp::DeclarationCategory::TYPE_PARAM, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::TypeAlias => (tsp::DeclarationCategory::TYPE_ALIAS, tsp::DeclarationFlags::new()),
                            _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member()),
                        }
                    },
                    _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                };

                // Extract module name from the definition's module (where the symbol is actually defined)
                let definition_module_name = definition.module_info.name();
                let module_parts: Vec<String> = definition_module_name.as_str().split('.').map(|s| s.to_string()).collect();
                let module_name = tsp::ModuleName {
                    leading_dots: 0,
                    name_parts: module_parts.clone(),
                };

                // Check if this is from builtins and update category/flags accordingly
                let (category, flags) = if module_parts.first().map_or(false, |first| first == "builtins") {
                    match category {
                        tsp::DeclarationCategory::FUNCTION | 
                        tsp::DeclarationCategory::CLASS | 
                        tsp::DeclarationCategory::VARIABLE => {
                            (tsp::DeclarationCategory::INTRINSIC, flags)
                        },
                        _ => (category, flags),
                    }
                } else {
                    (category, flags)
                };

                // Create node pointing to the actual definition location using the same logic as goto_definition
                let definition_uri = module_info_to_uri(&definition.module_info);
                let definition_uri_final = definition_uri.unwrap_or_else(|| params.node.uri.clone());

                // Add the primary declaration
                decls.push(tsp::Declaration {
                    handle: declaration_handle,
                    category,
                    flags,
                    node: Some(tsp::Node {
                        uri: definition_uri_final.clone(),
                        start: u32::from(definition.range.start()) as i32,
                        length: u32::from(definition.range.end() - definition.range.start()) as i32,
                    }),
                    module_name,
                    name: name.clone(),
                    uri: definition_uri_final,
                });

                // Get synthesized types if available
                let mut synth_types = Vec::new();
                if let Some(type_info) = type_info {
                    synth_types.push(self.convert_and_register_type(type_info));
                }

                (name, decls, synth_types)
            } else {
                // If no definition found, try to get type information at least
                let name = params.name.unwrap_or_else(|| {
                    let start = position;
                    let end = TextSize::new((params.node.start + params.node.length) as u32);
                    module_info.code_at(TextRange::new(start, end)).to_string()
                });

                let mut synth_types = Vec::new();
                if let Some(type_info) = type_info {
                    synth_types.push(self.convert_and_register_type(type_info));
                } else {
                    // No definition found and no type information available
                    eprintln!("Warning: No symbol definition or type information found at position {} in {}", position.to_usize(), uri);
                    return Ok(None);
                }

                (name, Vec::new(), synth_types)
            };

        Ok(Some(tsp::Symbol {
            node: params.node,
            name: symbol_name,
            decls: declarations,
            synthesized_types,
        }))
    }

    fn resolve_import_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::ResolveImportDeclarationParams,
    ) -> Result<Option<tsp::Declaration>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Only resolve import declarations
        if params.decl.category != tsp::DeclarationCategory::IMPORT {
            // Return the same declaration if it's not an import
            return Ok(Some(params.decl));
        }

        // Parse the module name from the declaration
        let module_name = &params.decl.module_name;
        let import_name = &params.decl.name;

        // Construct the full module path
        let full_module_path = if module_name.name_parts.is_empty() {
            return Ok(None);
        } else {
            module_name.name_parts.join(".")
        };

        // Try to find the module and resolve the imported symbol
        let pyrefly_module_name = pyrefly_python::module_name::ModuleName::from_str(&full_module_path);
        
        // Get Python search paths from the current context
        // We need to get the search paths from the importing file's context
        let importing_uri = &params.decl.uri;

        let _importing_path = match importing_uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                return Ok(Some(tsp::Declaration {
                    handle: params.decl.handle,
                    category: params.decl.category,
                    flags: params.decl.flags.with_unresolved_import(),
                    node: params.decl.node,
                    module_name: params.decl.module_name,
                    name: params.decl.name,
                    uri: params.decl.uri.clone(),
                }));
            }
        };

        // Get all search paths using the centralized get_python_search_paths method
        let search_paths_params = tsp::GetPythonSearchPathsParams {
            from_uri: importing_uri.clone(),
            snapshot: params.snapshot,
        };
        let search_paths_result = self.get_python_search_paths(transaction, search_paths_params);
        let all_search_paths: Vec<PathBuf> = search_paths_result.into_iter()
            .filter_map(|path_str| PathBuf::from(path_str).canonicalize().ok())
            .collect();
        
        // Try to find the module file in all search paths
        let mut target_module_path = None;
        let module_path_components = full_module_path.replace('.', "/");
        
        for search_path in &all_search_paths {
            // Try as a regular .py file
            let potential_py_file = search_path.join(&module_path_components).with_extension("py");
            if potential_py_file.exists() {
                target_module_path = Some(pyrefly_python::module_path::ModulePath::filesystem(potential_py_file));
                break;
            }
            
            // Try as a package with __init__.py
            let potential_package_init = search_path.join(&module_path_components).join("__init__.py");
            if potential_package_init.exists() {
                target_module_path = Some(pyrefly_python::module_path::ModulePath::filesystem(potential_package_init));
                break;
            }
            
            // Try as a .pyi stub file
            let potential_pyi_file = search_path.join(&module_path_components).with_extension("pyi");
            if potential_pyi_file.exists() {
                target_module_path = Some(pyrefly_python::module_path::ModulePath::filesystem(potential_pyi_file));
                break;
            }
            
            // Try as a package with __init__.pyi stub
            let potential_package_pyi = search_path.join(&module_path_components).join("__init__.pyi");
            if potential_package_pyi.exists() {
                target_module_path = Some(pyrefly_python::module_path::ModulePath::filesystem(potential_package_pyi));
                break;
            }
        }
        
        let Some(module_path) = target_module_path else {
            // Module file not found, return unresolved import
            return Ok(Some(tsp::Declaration {
                handle: params.decl.handle,
                category: params.decl.category,
                flags: params.decl.flags.with_unresolved_import(),
                node: params.decl.node,
                module_name: params.decl.module_name,
                name: params.decl.name,
                uri: params.decl.uri,
            }));
        };

        // Get the configuration for this module
        let config = self.state.config_finder().python_file(
            pyrefly_module_name,
            &module_path,
        );

        // Create a handle for the target module
        let target_handle = crate::state::handle::Handle::new(
            pyrefly_module_name,
            module_path.clone(),
            config.get_sys_info(),
        );

        // Try to get module info for the target module
        let Some(target_module_info) = transaction.get_module_info(&target_handle) else {
            // Module not found, possibly an unresolved import
            return Ok(Some(tsp::Declaration {
                handle: params.decl.handle,
                category: params.decl.category,
                flags: params.decl.flags.with_unresolved_import(),
                node: params.decl.node,
                module_name: params.decl.module_name,
                name: params.decl.name,
                uri: params.decl.uri,
            }));
        };

        // Look for the specific symbol in the target module
        // We'll use find_definition with a synthetic position to locate the symbol
        // This is a simplified approach - a full implementation would need to:
        // 1. Parse the target module's AST to find all exports
        // 2. Check __all__ if it exists
        // 3. Handle star imports properly
        // 4. Respect visibility rules (private vs public symbols)
        
        // Try to find all identifiers in the target module that match our import name
        // For simplicity, we'll search through the module content for the symbol definition
        let module_content = target_module_info.contents();
        
        // Look for function, class, or variable definitions of the imported name
        let patterns = [
            format!("def {}(", import_name),      // Function definition
            format!("class {}(", import_name),    // Class definition  
            format!("class {}:", import_name),    // Class definition without inheritance
            format!("{} =", import_name),         // Variable assignment
        ];
        
        let mut found_position = None;
        for pattern in &patterns {
            if let Some(pos) = module_content.find(pattern) {
                found_position = Some(pos);
                break;
            }
        }

        // If we found the symbol, try to get its definition info
        if let Some(pos) = found_position {
            let text_pos = TextSize::new(pos as u32);
            if let Some(first_definition) = transaction.find_definition(&target_handle, text_pos, true).first() {
                let def_metadata = &first_definition.metadata;
                let def_info = &first_definition.location;
                let _docstring = &first_definition.docstring;
                
                // Create a resolved declaration with proper category and flags
                let (category, flags) = match &def_metadata {
                    crate::state::lsp::DefinitionMetadata::Variable(Some(symbol_kind)) => {
                        match symbol_kind {
                            pyrefly_python::symbol_kind::SymbolKind::Function => (tsp::DeclarationCategory::FUNCTION, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Class => (tsp::DeclarationCategory::CLASS, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Variable => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                            pyrefly_python::symbol_kind::SymbolKind::Constant => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_constant()),
                            _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                        }
                    },
                    _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
                };

                return Ok(Some(tsp::Declaration {
                    handle: tsp::TypeHandle::String(format!("resolved_{}_{}", full_module_path, import_name)),
                    category,
                    flags,
                    node: Some(tsp::Node {
                        uri: Url::from_file_path(target_module_info.path().as_path()).unwrap(),
                        start: u32::from(def_info.range.start()) as i32,
                        length: u32::from(def_info.range.end() - def_info.range.start()) as i32,
                    }),
                    module_name: tsp::ModuleName {
                        leading_dots: 0,
                        name_parts: target_module_info.name().as_str().split('.').map(|s| s.to_string()).collect(),
                    },
                    name: import_name.clone(),
                    uri: Url::from_file_path(target_module_info.path().as_path()).unwrap(),
                }));
            }
        }

        // Fallback: create a generic resolved declaration pointing to the target module
        let resolved_declaration = tsp::Declaration {
            handle: tsp::TypeHandle::String(format!("resolved_{}_{}", full_module_path, import_name)),
            category: tsp::DeclarationCategory::VARIABLE, // Default to variable since we couldn't determine the type
            flags: tsp::DeclarationFlags::new(),
            node: None, // We don't have the exact location in the target module
            module_name: tsp::ModuleName {
                leading_dots: 0,
                name_parts: target_module_info.name().as_str().split('.').map(|s| s.to_string()).collect(),
            },
            name: import_name.clone(),
            uri: Url::from_file_path(target_module_info.path().as_path()).unwrap(), // Convert module path to URI
        };

        Ok(Some(resolved_declaration))
    }

    fn get_python_search_paths(&self, _transaction: &Transaction<'_>, params: tsp::GetPythonSearchPathsParams) -> Vec<String> {
        // Get the URI directly from params
        let uri = &params.from_uri;

        // Convert URI to file path
        let path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => return Vec::new(), // Return empty vector on error
        };

        // Check if language services are disabled for this workspace
        let workspace_disabled = self.workspaces.get_with(path.clone(), |workspace| {
            workspace.disable_language_services
        });

        if workspace_disabled {
            return Vec::new();
        }

        // Try to get configuration from config finder first
        let config_opt = if path.is_dir() {
            // For directories, use the directory method directly
            self.state.config_finder().directory(&path)
        } else {
            // For files, try to get config from python_file method
            let module_path = if self.open_files.read().contains_key(&path) {
                pyrefly_python::module_path::ModulePath::memory(path.clone())
            } else {
                pyrefly_python::module_path::ModulePath::filesystem(path.clone())
            };
            
            // python_file always returns a config, but check if it's synthetic
            let config = self.state.config_finder().python_file(
                pyrefly_python::module_name::ModuleName::unknown(),
                &module_path,
            );
            
            // If it's a real config file (not synthetic), use it
            match &config.source {
                crate::config::config::ConfigSource::File(_) 
                | crate::config::config::ConfigSource::Marker(_) => Some(config),
                crate::config::config::ConfigSource::Synthetic => None,
            }
        };

        if let Some(config) = config_opt {
            // We found a real config file, use its search paths
            let mut search_paths = Vec::new();
            
            // Add search paths from config
            for path in config.search_path() {
                if let Some(path_str) = path.to_str() {
                    search_paths.push(path_str.to_string());
                }
            }

            // Add site package paths from config
            for path in config.site_package_path() {
                if let Some(path_str) = path.to_str() {
                    search_paths.push(path_str.to_string());
                }
            }

            search_paths
        } else {
            // No config file found, use workspace python_info as fallback
            self.workspaces.get_with(path, |workspace| {
                let mut search_paths = Vec::new();

                // Add workspace-specific search paths if available
                if let Some(workspace_search_paths) = &workspace.search_path {
                    for path in workspace_search_paths {
                        if let Some(path_str) = path.to_str() {
                            search_paths.push(path_str.to_string());
                        }
                    }
                }

                // Add search paths from workspace python environment
                if let Some(python_info) = &workspace.python_info {
                    // Add site package paths from the python environment
                    if let Some(site_packages) = &python_info.env.site_package_path {
                        for path in site_packages {
                            if let Some(path_str) = path.to_str() {
                                search_paths.push(path_str.to_string());
                            }
                        }
                    }

                    // Add interpreter site package paths from the python environment
                    for path in &python_info.env.interpreter_site_package_path {
                        if let Some(path_str) = path.to_str() {
                            search_paths.push(path_str.to_string());
                        }
                    }
                }

                search_paths
            })
        }
    }

    /// Load a module in a fresh transaction if it's not available in the current transaction
    fn load_module_if_needed(
        &self,
        transaction: &Transaction<'_>,
        handle: &crate::state::handle::Handle,
    ) -> Option<Transaction> {
        // Check if module is already loaded in the current transaction
        if transaction.get_module_info(handle).is_some() {
            return None; // Module already loaded, no need for fresh transaction
        }

        // Module not loaded, create a fresh transaction and load it
        let mut fresh_transaction = self.state.transaction();
        fresh_transaction.run(&[(handle.clone(), crate::state::require::Require::Everything)]);
        
        // Verify the module was loaded successfully
        if fresh_transaction.get_module_info(handle).is_some() {
            Some(fresh_transaction)
        } else {
            None // Module couldn't be loaded even with fresh transaction
        }
    }

    /// Try to get type information for an import declaration by resolving the import
    fn get_type_for_import_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: &tsp::GetTypeOfDeclarationParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        let resolve_params = tsp::ResolveImportDeclarationParams {
            decl: params.decl.clone(),
            options: tsp::ResolveImportOptions::default(),
            snapshot: params.snapshot,
        };
        
        if let Ok(Some(resolved_decl)) = self.resolve_import_declaration(transaction, resolve_params) {
            if let Some(resolved_node) = &resolved_decl.node {
                let resolved_uri = &resolved_node.uri;
                
                if let Some(resolved_handle) = self.make_handle_if_enabled(resolved_uri) {
                    let resolved_position = TextSize::new(resolved_node.start as u32);
                    if let Some(resolved_type) = transaction.get_type_at(&resolved_handle, resolved_position) {
                        return Ok(Some(self.convert_and_register_type(resolved_type)));
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Try to get type information for an import declaration using a fresh transaction
    fn get_type_for_import_declaration_with_fresh_transaction(
        &self,
        fresh_transaction: &mut Transaction,
        params: &tsp::GetTypeOfDeclarationParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        let resolve_params = tsp::ResolveImportDeclarationParams {
            decl: params.decl.clone(),
            options: tsp::ResolveImportOptions::default(),
            snapshot: params.snapshot,
        };
        
        if let Ok(Some(resolved_decl)) = self.resolve_import_declaration(fresh_transaction, resolve_params) {
            if let Some(resolved_node) = &resolved_decl.node {
                let resolved_uri = &resolved_node.uri;
                
                if let Some(resolved_handle) = self.make_handle_if_enabled(resolved_uri) {
                    // Make sure the resolved module is also loaded
                    fresh_transaction.run(&[(resolved_handle.clone(), crate::state::require::Require::Everything)]);
                    
                    let resolved_position = TextSize::new(resolved_node.start as u32);
                    if let Some(resolved_type) = fresh_transaction.get_type_at(&resolved_handle, resolved_position) {
                        return Ok(Some(self.convert_and_register_type(resolved_type)));
                    }
                }
            }
        }
        
        Ok(None)
    }

    fn get_type_of_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeOfDeclarationParams,
    ) -> Result<tsp::Type, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Extract the location information from the declaration
        let Some(node) = &params.decl.node else {
            // If there's no node information, we can't get the type
            return Err(ResponseError {
                code: ErrorCode::InvalidParams as i32,
                message: "Declaration has no node information".to_string(),
                data: None,
            });
        };

        // Convert Node URI to a handle
        let uri = &node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_string(),
                data: None,
            });
        };

        // Get module info for position conversion
        // Note: If the module is not loaded in the transaction, we'll load it ourselves
        // If we can't get module info, the file might not be loaded in the transaction
        // This can happen when the declaration points to a definition in a file that's not currently loaded
        let _module_info = match transaction.get_module_info(&handle) {
            Some(info) => info,
            None => {
                // Module not loaded in transaction, try to load it
                let Some(mut fresh_transaction) = self.load_module_if_needed(transaction, &handle) else {
                    // If we still can't load the module, fall back to default type
                    return Ok(create_default_type_for_declaration(&params.decl));
                };
                
                // Convert declaration position to TextSize
                let position = TextSize::new(node.start as u32);
                
                // Try to get the type at the declaration's position using the fresh transaction
                let Some(type_info) = fresh_transaction.get_type_at(&handle, position) else {
                    // If we can't get type info from the position, try alternative approaches
                    
                    // For imports, we might need to resolve the imported symbol first
                    if params.decl.category == tsp::DeclarationCategory::IMPORT {
                        if let Ok(Some(import_type)) = self.get_type_for_import_declaration_with_fresh_transaction(&mut fresh_transaction, &params) {
                            return Ok(import_type);
                        }
                    }
                    
                    // If still no type found, create a generic type based on the declaration category
                    return Ok(create_default_type_for_declaration(&params.decl));
                };
                
                // Convert pyrefly Type to TSP Type format using the fresh transaction result
                return Ok(self.convert_and_register_type(type_info));
            }
        };

        // Convert declaration position to TextSize
        let position = TextSize::new(node.start as u32);

        // Try to get the type at the declaration's position
        let Some(type_info) = transaction.get_type_at(&handle, position) else {
            // If we can't get type info from the position, try alternative approaches
            
            // For imports, we might need to resolve the imported symbol first
            if params.decl.category == tsp::DeclarationCategory::IMPORT {
                if let Ok(Some(import_type)) = self.get_type_for_import_declaration(transaction, &params) {
                    return Ok(import_type);
                }
            }
            
            // If still no type found, create a generic type based on the declaration category
            return Ok(create_default_type_for_declaration(&params.decl));
        };

        // Convert pyrefly Type to TSP Type format
        Ok(self.convert_and_register_type(type_info))
    }

    fn get_repr(
        &self,
        _transaction: &Transaction<'_>,
        params: tsp::GetReprParams,
    ) -> Result<String, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Use the handle mapping to get the actual pyrefly type
        let Some(internal_type) = self.lookup_type_from_tsp_type(&params.type_param) else {
            // If we can't find the internal type, fall back to the basic formatter
            eprintln!("Warning: Could not resolve type handle for repr: {:?}", params.type_param.handle);
            let type_repr = format_type_representation(&params.type_param, params.flags);
            return Ok(type_repr);
        };

        // Use pyrefly's native type formatting
        let type_repr = if params.flags.has_convert_to_instance_type() {
            // Convert class types to instance types
            match &internal_type {
                crate::types::types::Type::ClassDef(class) => {
                    // Convert ClassDef to ClassType (instance)
                    let empty_tparams = std::sync::Arc::new(crate::types::types::TParams::new(Vec::new()));
                    let empty_targs = crate::types::types::TArgs::new(empty_tparams, Vec::new());
                    let class_type = crate::types::class::ClassType::new(
                        class.clone(),
                        empty_targs,
                    );
                    format!("{}", crate::types::types::Type::ClassType(class_type))
                }
                _ => format!("{}", internal_type)
            }
        } else {
            // Standard type representation
            format!("{}", internal_type)
        };

        // Apply additional formatting based on flags
        let final_repr = if params.flags.has_expand_type_aliases() {
            // For now, we don't have specific alias expansion logic in the Display impl,
            // but this is where we would implement it if needed
            type_repr
        } else {
            type_repr
        };

        eprintln!("Generated repr for type {:?}: {}", params.type_param.handle, final_repr);
        Ok(final_repr)
    }

    fn get_docstring(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetDocstringParams,
    ) -> Result<Option<String>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Extract the location information from the declaration
        let Some(node) = &params.decl.node else {
            // If there's no node information, we can't find the docstring
            return Ok(None);
        };

        // Convert Node URI to a handle
        let uri = &node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Ok(None);
        };

        // Get module info for position conversion
        let Some(_module_info) = transaction.get_module_info(&handle) else {
            // If module not loaded in transaction, try to load it
            let Some(fresh_transaction) = self.load_module_if_needed(transaction, &handle) else {
                return Ok(None);
            };
            
            return self.extract_docstring_from_transaction(&fresh_transaction, &handle, node);
        };

        // Use the current transaction to find the docstring
        self.extract_docstring_from_transaction(transaction, &handle, node)
    }

    /// Helper method to extract docstring from a transaction, reusing hover logic
    fn extract_docstring_from_transaction(
        &self,
        transaction: &Transaction<'_>,
        handle: &crate::state::handle::Handle,
        node: &tsp::Node,
    ) -> Result<Option<String>, ResponseError> {
        // Convert position to TextSize
        let position = TextSize::new(node.start as u32);

        // Try to find definition at the position - this is the same logic as hover
        if let Some(first_definition) = transaction.find_definition(handle, position, true).first() {
            let _definition_metadata = &first_definition.metadata;
            let _text_range_with_module_info = &first_definition.location;
            let docstring = &first_definition.docstring;
            
            if let Some(docstring) = docstring {
                return Ok(Some(docstring.as_string().trim().to_string()));
            }
        }

        // No docstring found
        Ok(None)
    }

    fn search_for_type_attribute(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::SearchForTypeAttributeParams,
    ) -> Result<Option<tsp::Attribute>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        eprintln!(
            "Searching for attribute '{}' with access flags: {:?}",
            params.attribute_name, params.access_flags
        );

        // Get the internal type from the start_type handle
        let internal_type = match self.lookup_type_from_tsp_type(&params.start_type) {
            Some(t) => t,
            None => {
                eprintln!("Could not resolve type handle: {:?}", params.start_type.handle);
                return Ok(None);
            }
        };

        // Only work on class types - this method is specifically for class attribute lookup
        match &internal_type {
            crate::types::types::Type::ClassType(class_type) => {
                self.search_attribute_in_class_type(class_type, &params.attribute_name, transaction, &params)
            }
            _ => {
                eprintln!(
                    "search_for_type_attribute only works on class types, got: {:?}",
                    internal_type
                );
                Ok(None)
            }
        }
    }

    /// Search for attribute in class types using the solver
    fn search_attribute_in_class_type(
        &self,
        class_type: &crate::types::class::ClassType,
        attribute_name: &str,
        transaction: &Transaction,
        _params: &tsp::SearchForTypeAttributeParams,
    ) -> Result<Option<tsp::Attribute>, ResponseError> {
        eprintln!(
            "Searching for attribute '{}' in class type using solver",
            attribute_name
        );

        // Convert string attribute name to ruff_python_ast::name::Name
        let attr_name = Name::new(attribute_name.to_string());

        // Get the module info from the class type
        let class_qname = class_type.class_object().qname();
        let module_info = class_qname.module_info();
        
        // Create a handle for the module containing the class
        // Use the existing config finder to get the proper config
        let module_path = module_info.path().clone();
        let module_name = module_info.name();
        let config = self.state.config_finder().python_file(module_name, &module_path);
        let handle = crate::state::handle::Handle::new(
            module_name,
            module_path,
            config.get_sys_info(),
        );

        // Use ad_hoc_solve to access the solver and look up the attribute
        let result = transaction.ad_hoc_solve(&handle, |solver| {
            // First try to get the attribute using try_lookup_attr_from_class_type
            if let Some(attribute) = solver.try_lookup_attr_from_class_type(class_type.clone(), &attr_name) {
                // We found the attribute, now we need to get its type
                // Use type_of_attr_get to get the resolved type
                let class_instance_type = crate::types::types::Type::ClassType(class_type.clone());
                let attribute_type = solver.type_of_attr_get(
                    &class_instance_type,
                    &attr_name,
                    ruff_text_size::TextRange::default(), // Use a default range
                    &ErrorCollector::new(
                        module_info.clone(),
                        ErrorStyle::Never,
                    ), // Create a temporary error collector
                    None, // No context
                    "search_for_type_attribute", // Context description
                );
                
                Some((attribute, attribute_type))
            } else {
                None
            }
        });

        match result {
            Some(Some((_attribute, attribute_type))) => {
                eprintln!(
                    "Found attribute '{}' in class type with type: {:?}",
                    attribute_name, attribute_type
                );
                
                // Convert the pyrefly Attribute to TSP Attribute
                let tsp_attribute = self.convert_type_to_tsp_attribute(attribute_type, attribute_name);
                Ok(Some(tsp_attribute))
            }
            Some(None) => {
                eprintln!(
                    "Attribute '{}' not found in class type",
                    attribute_name
                );
                Ok(None)
            }
            None => {
                eprintln!(
                    "Failed to create solver for attribute lookup of '{}'",
                    attribute_name
                );
                Ok(None)
            }
        }
    }

    /// Convert a pyrefly Type to TSP Attribute format
    fn convert_type_to_tsp_attribute(
        &self,
        attribute_type: crate::types::types::Type,
        attribute_name: &str,
    ) -> tsp::Attribute {
        // Convert the pyrefly type to TSP type and register it
        let tsp_type = self.convert_and_register_type(attribute_type);

        // For now, create default flags - we could enhance this later with more attribute metadata
        let flags = tsp::AttributeFlags::NONE;

        tsp::Attribute {
            name: attribute_name.to_string(),
            type_info: tsp_type,
            owner: None, // TODO: Could set this to the class type if needed
            bound_type: None, // TODO: Could implement bound type if needed
            flags,
            decls: Vec::new(), // TODO: Could add declaration information if available
        }
    }

    fn current_snapshot(&self) -> i32 {
        self.state.current_snapshot()
    }

    /// Converts a pyrefly type to TSP type and registers it in the lookup table
    fn convert_and_register_type(&self, py_type: crate::types::types::Type) -> tsp::Type {
        let tsp_type = tsp::convert_to_tsp_type(py_type.clone());
        
        // Register the type in the lookup table
        if let tsp::TypeHandle::String(handle_str) = &tsp_type.handle {
            self.register_type_handle(handle_str.clone(), py_type);
        }
        
        tsp_type
    }

    /// Registers a type handle in the lookup table, clearing old entries if snapshot changed
    fn register_type_handle(&self, handle: String, py_type: crate::types::types::Type) {
        let current_snapshot = self.current_snapshot();
        let mut lookup = self.type_handle_lookup.write();
        
        // Clear the lookup table if snapshot has changed
        if lookup.0 != current_snapshot {
            lookup.1.clear();
            lookup.0 = current_snapshot;
        }
        
        lookup.1.insert(handle, py_type);
    }

    /// Looks up a pyrefly type from a TSP type handle
    fn lookup_type_from_handle(&self, handle: &str) -> Option<crate::types::types::Type> {
        let current_snapshot = self.current_snapshot();
        let mut lookup = self.type_handle_lookup.write();
        
        // Clear the lookup table if snapshot has changed
        if lookup.0 != current_snapshot {
            lookup.1.clear();
            lookup.0 = current_snapshot;
            return None;
        }
        
        lookup.1.get(handle).cloned()
    }

    /// Looks up a pyrefly type from an integer TSP type handle
    fn lookup_type_by_int_handle(&self, id: i32) -> Option<crate::types::types::Type> {
        // For now, convert integer handle to string and use the existing lookup
        // In a more sophisticated implementation, we might have separate integer and string lookups
        let handle_str = format!("{}", id);
        self.lookup_type_from_handle(&handle_str)
    }

    /// Looks up a pyrefly type from a TSP Type
    fn lookup_type_from_tsp_type(&self, tsp_type: &tsp::Type) -> Option<crate::types::types::Type> {
        match &tsp_type.handle {
            tsp::TypeHandle::String(handle_str) => self.lookup_type_from_handle(handle_str),
            tsp::TypeHandle::Integer(id) => self.lookup_type_by_int_handle(*id),
        }
    }

    pub fn categorized_events(events: Vec<lsp_types::FileEvent>) -> CategorizedEvents {
        let mut created = Vec::new();
        let mut modified = Vec::new();
        let mut removed = Vec::new();
        let mut unknown = Vec::new();

        for event in events {
            match event.typ {
                lsp_types::FileChangeType::CREATED => {
                    created.push(event.uri.to_file_path().unwrap());
                }
                lsp_types::FileChangeType::CHANGED => {
                    modified.push(event.uri.to_file_path().unwrap());
                }
                lsp_types::FileChangeType::DELETED => {
                    removed.push(event.uri.to_file_path().unwrap());
                }
                _ => {
                    unknown.push(event.uri.to_file_path().unwrap());
                }
            }
        }

        CategorizedEvents {
            created,
            modified,
            removed,
            unknown,
        }
    }

    fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) -> anyhow::Result<()> {
        if !params.changes.is_empty() {
            self.invalidate(move |t| {
                t.invalidate_events(&Self::categorized_events(params.changes))
            });
        }
        // rewatch files in case we loaded or dropped any configs
        self.setup_file_watcher_if_necessary(
            self.workspaces
                .workspaces
                .read()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .as_slice(),
        );
        Ok(())
    }

    fn did_close(&self, params: DidCloseTextDocumentParams) -> anyhow::Result<()> {
        let uri = params.text_document.uri.to_file_path().unwrap();
        self.version_info.lock().remove(&uri);
        self.open_files.write().remove(&uri);
        self.connection
            .publish_diagnostics_for_uri(params.text_document.uri, Vec::new(), None);
        Ok(())
    }

    fn workspace_folders_changed(&self, params: DidChangeWorkspaceFoldersParams) {
        let removed = params
            .event
            .removed
            .iter()
            .map(|x| x.uri.to_file_path().unwrap())
            .collect::<Vec<_>>();
        let added = params
            .event
            .added
            .iter()
            .map(|x| x.uri.to_file_path().unwrap())
            .collect::<Vec<_>>();

        self.state.increment_snapshot(); // Increment snapshot on workspace change
        self.configure(&added, &removed);
    }

    fn did_change_configuration<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        params: DidChangeConfigurationParams,
    ) -> anyhow::Result<()> {
        if let Some(workspace) = &self.initialize_params.capabilities.workspace
            && workspace.configuration == Some(true)
        {
            self.request_settings_for_all_workspaces();
            return Ok(());
        }

        let mut modified = false;
        if let Some(python) = params.settings.get(PYTHON_SECTION) {
            let config: LspConfig = serde_json::from_value(python.clone())?;
            self.apply_client_configuration(&mut modified, &None, config);
        }

        if modified {
            self.validate_in_memory(ide_transaction_manager)?;
        }
        Ok(())
    }

    /// Configure the server with a new set of workspace folders
    fn configure(&self, workspace_paths_added: &[PathBuf], workspace_paths_removed: &[PathBuf]) {
        let mut all_workspaces = Vec::new();
        {
            let mut workspaces = self.workspaces.workspaces.write();
            for x in workspace_paths_added {
                workspaces.insert(x.clone(), Workspace::new_with_default_env(x));
            }
            for x in workspace_paths_removed {
                workspaces.shift_remove(x);
            }
            workspaces
                .keys()
                .for_each(|uri| all_workspaces.push(uri.clone()));
        }
        self.setup_file_watcher_if_necessary(all_workspaces.as_slice());
        self.request_settings_for_all_workspaces();
    }

    fn handle_from_module_path(state: &State, path: ModulePath) -> Handle {
        let unknown = ModuleName::unknown();
        let config = state.config_finder().python_file(unknown, &path);
        let module_name = to_real_path(&path)
            .and_then(|path| module_from_path(&path, config.search_path()))
            .unwrap_or(unknown);
        Handle::new(module_name, path, config.get_sys_info())
    }

    fn make_open_handle(state: &State, path: &Path) -> Handle {
        let path = ModulePath::memory(path.to_owned());
        Self::handle_from_module_path(state, path)
    }

    /// Create a handle. Return None if the workspace has language services disabled (and thus you shouldn't do anything).
    fn make_handle_if_enabled(&self, uri: &Url) -> Option<Handle> {
        let path = uri.to_file_path().unwrap();
        self.workspaces.get_with(path.clone(), |workspace| {
            if workspace.disable_language_services {
                eprintln!("Skipping request - language services disabled");
                None
            } else {
                let module_path = if self.open_files.read().contains_key(&path) {
                    ModulePath::memory(path)
                } else {
                    ModulePath::filesystem(path)
                };
                Some(Self::handle_from_module_path(&self.state, module_path))
            }
        })
    }

    fn to_lsp_location(&self, location: &TextRangeWithModuleInfo) -> Option<Location> {
        let TextRangeWithModuleInfo {
            module_info: definition_module_info,
            range,
        } = location;
        let uri = module_info_to_uri(definition_module_info)?;
        Some(Location {
            uri,
            range: definition_module_info.lined_buffer().to_lsp_range(*range),
        })
    }

    fn goto_definition(
        &self,
        transaction: &Transaction<'_>,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let uri = &params.text_document_position_params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let range = info
            .lined_buffer()
            .from_lsp_position(params.text_document_position_params.position);
        let targets = transaction.goto_definition(&handle, range);
        let mut lsp_targets = targets
            .into_iter()
            .filter_map(|t| self.to_lsp_location(&t))
            .collect::<Vec<_>>();
        if lsp_targets.is_empty() {
            None
        } else if lsp_targets.len() == 1 {
            Some(GotoDefinitionResponse::Scalar(lsp_targets.pop().unwrap()))
        } else {
            Some(GotoDefinitionResponse::Array(lsp_targets))
        }
    }

    fn completion(
        &self,
        transaction: &Transaction<'_>,
        params: CompletionParams,
    ) -> anyhow::Result<CompletionResponse> {
        let uri = &params.text_document_position.text_document.uri;
        let handle = match self.make_handle_if_enabled(uri) {
            None => {
                return Ok(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: Vec::new(),
                }));
            }
            Some(x) => x,
        };
        let items = transaction
            .get_module_info(&handle)
            .map(|info| {
                transaction.completion(
                    &handle,
                    info.lined_buffer()
                        .from_lsp_position(params.text_document_position.position),
                )
            })
            .unwrap_or_default();
        Ok(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
        }))
    }

    fn code_action(
        &self,
        transaction: &Transaction<'_>,
        params: CodeActionParams,
    ) -> Option<CodeActionResponse> {
        let uri = &params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let module_info = transaction.get_module_info(&handle)?;
        let range = module_info.lined_buffer().from_lsp_range(params.range);
        let code_actions = transaction
            .local_quickfix_code_actions(&handle, range)?
            .into_map(|(title, info, range, insert_text)| {
                CodeActionOrCommand::CodeAction(CodeAction {
                    title,
                    kind: Some(CodeActionKind::QUICKFIX),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from([(
                            uri.clone(),
                            vec![TextEdit {
                                range: info.lined_buffer().to_lsp_range(range),
                                new_text: insert_text,
                            }],
                        )])),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            });
        Some(code_actions)
    }

    fn document_highlight(
        &self,
        transaction: &Transaction<'_>,
        params: DocumentHighlightParams,
    ) -> Option<Vec<DocumentHighlight>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let position = info
            .lined_buffer()
            .from_lsp_position(params.text_document_position_params.position);
        Some(
            transaction
                .find_local_references(&handle, position)
                .into_map(|range| DocumentHighlight {
                    range: info.lined_buffer().to_lsp_range(range),
                    kind: None,
                }),
        )
    }

    /// Compute references of a symbol at a given position. This is a non-blocking function, the
    /// it will send a response to the LSP client once the results are found and transformed by
    /// `map_result`.
    fn async_find_references_helper<'a, V: serde::Serialize>(
        &'a self,
        request_id: RequestId,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        uri: &Url,
        position: Position,
        map_result: impl FnOnce(Vec<(Url, Vec<Range>)>) -> V + Send + 'static,
    ) {
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return self.send_response(new_response::<Option<V>>(request_id, Ok(None)));
        };
        let transaction = ide_transaction_manager.non_commitable_transaction(&self.state);
        let Some(info) = transaction.get_module_info(&handle) else {
            ide_transaction_manager.save(transaction);
            return self.send_response(new_response::<Option<V>>(request_id, Ok(None)));
        };
        let position = info.lined_buffer().from_lsp_position(position);
        let Some(FindDefinitionItem {
            metadata,
            location,
            docstring: _,
        }) = transaction
            .find_definition(&handle, position, false)
            // TODO: handle more than 1 definition
            .into_iter()
            .next()
        else {
            ide_transaction_manager.save(transaction);
            return self.send_response(new_response::<Option<V>>(request_id, Ok(None)));
        };
        ide_transaction_manager.save(transaction);
        let state = self.state.dupe();
        let open_files = self.open_files.dupe();
        let cancellation_handles = self.cancellation_handles.dupe();

        let connection = self.connection.dupe();
        self.async_state_read_threads.async_spawn(move || {
            let mut transaction = state.cancellable_transaction();
            cancellation_handles
                .lock()
                .insert(request_id.clone(), transaction.get_cancellation_handle());
            Self::validate_in_memory_for_transaction(&state, &open_files, transaction.as_mut());
            match transaction.find_global_references_from_definition(
                handle.sys_info(),
                metadata,
                location,
            ) {
                Ok(global_references) => {
                    let mut locations = Vec::new();
                    for (info, ranges) in global_references {
                        if let Some(uri) = module_info_to_uri(&info) {
                            locations.push((
                                uri,
                                ranges.into_map(|range| info.lined_buffer().to_lsp_range(range)),
                            ));
                        };
                    }
                    connection.send(Message::Response(new_response(
                        request_id,
                        Ok(Some(map_result(locations))),
                    )));
                }
                Err(Cancelled) => {
                    let message = format!("Find reference request {request_id} is canceled");
                    eprintln!("{message}");
                    connection.send(Message::Response(Response::new_err(
                        request_id,
                        ErrorCode::RequestCanceled as i32,
                        message,
                    )))
                }
            }
        });
    }

    fn references<'a>(
        &'a self,
        request_id: RequestId,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        params: ReferenceParams,
    ) {
        self.async_find_references_helper(
            request_id,
            ide_transaction_manager,
            &params.text_document_position.text_document.uri,
            params.text_document_position.position,
            move |results| {
                let mut locations = Vec::new();
                for (uri, ranges) in results {
                    for range in ranges {
                        locations.push(Location {
                            uri: uri.clone(),
                            range,
                        })
                    }
                }
                locations
            },
        );
    }

    fn rename<'a>(
        &'a self,
        request_id: RequestId,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        params: RenameParams,
    ) {
        self.async_find_references_helper(
            request_id,
            ide_transaction_manager,
            &params.text_document_position.text_document.uri,
            params.text_document_position.position,
            move |results| {
                let mut changes = HashMap::new();
                for (uri, ranges) in results {
                    changes.insert(
                        uri,
                        ranges.into_map(|range| TextEdit {
                            range,
                            new_text: params.new_name.clone(),
                        }),
                    );
                }
                WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }
            },
        );
    }

    fn prepare_rename(
        &self,
        transaction: &Transaction<'_>,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        let uri = &params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let position = info.lined_buffer().from_lsp_position(params.position);
        transaction
            .prepare_rename(&handle, position)
            .map(|range| PrepareRenameResponse::Range(info.lined_buffer().to_lsp_range(range)))
    }

    fn signature_help(
        &self,
        transaction: &Transaction<'_>,
        params: SignatureHelpParams,
    ) -> Option<SignatureHelp> {
        let uri = &params.text_document_position_params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let position = info
            .lined_buffer()
            .from_lsp_position(params.text_document_position_params.position);
        transaction.get_signature_help_at(&handle, position)
    }

    fn hover(&self, transaction: &Transaction<'_>, params: HoverParams) -> Option<Hover> {
        let uri = &params.text_document_position_params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let range = info
            .lined_buffer()
            .from_lsp_position(params.text_document_position_params.position);
        let t = transaction.get_type_at(&handle, range)?;
        let mut kind_formatted: String = "".to_owned();
        let mut docstring_formatted: String = "".to_owned();
        if let Some(FindDefinitionItem {
            metadata,
            location,
            docstring,
        }) = transaction
            .find_definition(&handle, range, true)
            // TODO: handle more than 1 definition
            .into_iter()
            .next()
        {
            if let Some(symbol_kind) = metadata.symbol_kind() {
                kind_formatted = format!(
                    "{} {}: ",
                    &symbol_kind.display_for_hover(),
                    location.module_info.code_at(location.range)
                );
            }
            if let Some(docstring) = docstring {
                docstring_formatted = format!("\n---\n{}", docstring.as_string().trim());
            }
        }
        let mut type_display_context = TypeDisplayContext::new(&[&t]);
        type_display_context.start_tracking_displayed_class_definitions();
        let type_formatted = format!("{}", type_display_context.display(&t));
        let symbol_def_loc_formatted = {
            let tracked_def_locs = type_display_context
                .tracked_displayed_class_definitions()
                .unwrap_or_default();
            let linked_names = tracked_def_locs
                .into_iter()
                .filter_map(|qname| {
                    if let Ok(mut url) = Url::from_file_path(qname.module_info().path().as_path()) {
                        let start_pos = qname.module_info().display_range(qname.range()).start;
                        url.set_fragment(Some(&format!(
                            "L{},{}",
                            start_pos.line.get(),
                            start_pos.column
                        )));
                        Some(format!("[{}]({})", qname.id(), url))
                    } else {
                        None
                    }
                })
                .join(" | ");
            if linked_names.is_empty() {
                "".to_owned()
            } else {
                format!("\n---\nGo to {linked_names}")
            }
        };
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "```python\n{kind_formatted}{type_formatted}\n```{docstring_formatted}{symbol_def_loc_formatted}",
                ),
            }),
            range: None,
        })
    }

    fn inlay_hints(
        &self,
        transaction: &Transaction<'_>,
        params: InlayHintParams,
    ) -> Option<Vec<InlayHint>> {
        let uri = &params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let info = transaction.get_module_info(&handle)?;
        let t = transaction.inlay_hints(&handle)?;
        Some(t.into_map(|x| {
            let position = info.lined_buffer().to_lsp_position(x.0);
            InlayHint {
                position,
                label: InlayHintLabel::String(x.1.clone()),
                kind: None,
                text_edits: Some(vec![TextEdit {
                    range: Range::new(position, position),
                    new_text: x.1,
                }]),
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            }
        }))
    }

    fn semantic_tokens_full(
        &self,
        transaction: &Transaction<'_>,
        params: SemanticTokensParams,
    ) -> Option<SemanticTokensResult> {
        let uri = &params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: transaction
                .semantic_tokens(&handle, None)
                .unwrap_or_default(),
        }))
    }

    fn semantic_tokens_ranged(
        &self,
        transaction: &Transaction<'_>,
        params: SemanticTokensRangeParams,
    ) -> Option<SemanticTokensRangeResult> {
        let uri = &params.text_document.uri;
        let handle = self.make_handle_if_enabled(uri)?;
        let module_info = transaction.get_module_info(&handle)?;
        let range = module_info.lined_buffer().from_lsp_range(params.range);
        Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: transaction
                .semantic_tokens(&handle, Some(range))
                .unwrap_or_default(),
        }))
    }

    fn hierarchical_document_symbols(
        &self,
        transaction: &Transaction<'_>,
        params: DocumentSymbolParams,
    ) -> Option<Vec<DocumentSymbol>> {
        let uri = &params.text_document.uri;
        if self
            .workspaces
            .get_with(uri.to_file_path().unwrap(), |workspace| {
                workspace.disable_language_services
            })
            || !self
                .initialize_params
                .capabilities
                .text_document
                .as_ref()?
                .document_symbol
                .as_ref()?
                .hierarchical_document_symbol_support?
        {
            return None;
        }
        let handle = self.make_handle_if_enabled(uri)?;
        transaction.symbols(&handle)
    }

    #[allow(deprecated)] // The `deprecated` field
    fn workspace_symbols(
        &self,
        transaction: &Transaction<'_>,
        query: &str,
    ) -> Vec<SymbolInformation> {
        transaction
            .workspace_symbols(query)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(name, kind, location)| {
                self.to_lsp_location(&location)
                    .map(|location| SymbolInformation {
                        name,
                        kind,
                        location,
                        tags: None,
                        deprecated: None,
                        container_name: None,
                    })
            })
            .collect()
    }

    fn document_diagnostics(
        &self,
        transaction: &Transaction<'_>,
        params: DocumentDiagnosticParams,
    ) -> DocumentDiagnosticReport {
        let handle = Self::make_open_handle(
            &self.state,
            &params.text_document.uri.to_file_path().unwrap(),
        );
        let mut items = Vec::new();
        let open_files = &self.open_files.read();
        for e in transaction.get_errors(once(&handle)).collect_errors().shown {
            if let Some((_, diag)) = self.get_diag_if_shown(&e, open_files) {
                items.push(diag);
            }
        }
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                items,
                result_id: None,
            },
            related_documents: None,
        })
    }

    fn get_pattern_to_watch(
        root: &Path,
        pattern: String,
        relative_pattern_support: bool,
    ) -> GlobPattern {
        if relative_pattern_support && let Ok(url) = Url::from_directory_path(root) {
            GlobPattern::Relative(RelativePattern {
                base_uri: OneOf::Right(url),
                pattern,
            })
        } else {
            GlobPattern::String(root.join(pattern).to_string_lossy().into_owned())
        }
    }

    fn setup_file_watcher_if_necessary(&self, roots: &[PathBuf]) {
        match self.initialize_params.capabilities.workspace {
            Some(WorkspaceClientCapabilities {
                did_change_watched_files:
                    Some(DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(true),
                        relative_pattern_support,
                        ..
                    }),
                ..
            }) => {
                let relative_pattern_support = relative_pattern_support.is_some_and(|b| b);
                if self.filewatcher_registered.load(Ordering::Relaxed) {
                    self.send_request::<UnregisterCapability>(UnregistrationParams {
                        unregisterations: Vec::from([Unregistration {
                            id: Self::FILEWATCHER_ID.to_owned(),
                            method: DidChangeWatchedFiles::METHOD.to_owned(),
                        }]),
                    });
                }
                // TODO(connernilsen): we need to dedup filewatcher patterns
                // preferably by figuring out if they're under another wildcard pattern with the same suffix
                let mut glob_patterns = Vec::new();
                for root in roots {
                    PYTHON_FILE_SUFFIXES_TO_WATCH.iter().for_each(|suffix| {
                        glob_patterns.push(Self::get_pattern_to_watch(
                            root,
                            format!("**/*.{suffix}"),
                            relative_pattern_support,
                        ));
                    });
                    ConfigFile::CONFIG_FILE_NAMES.iter().for_each(|config| {
                        glob_patterns.push(Self::get_pattern_to_watch(
                            root,
                            format!("**/{config}"),
                            relative_pattern_support,
                        ))
                    });
                }
                for (root, pattern) in self
                    .workspaces
                    .loaded_configs
                    .get_patterns_for_cached_configs()
                {
                    glob_patterns.push(Self::get_pattern_to_watch(
                        &root,
                        pattern,
                        relative_pattern_support,
                    ));
                }
                let watchers = glob_patterns
                    .into_iter()
                    .map(|glob_pattern| FileSystemWatcher {
                        glob_pattern,
                        kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
                    })
                    .collect::<Vec<_>>();
                self.send_request::<RegisterCapability>(RegistrationParams {
                    registrations: Vec::from([Registration {
                        id: Self::FILEWATCHER_ID.to_owned(),
                        method: DidChangeWatchedFiles::METHOD.to_owned(),
                        register_options: Some(
                            serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                                watchers,
                            })
                            .unwrap(),
                        ),
                    }]),
                });
                self.filewatcher_registered.store(true, Ordering::Relaxed);
            }
            _ => (),
        }
    }

    fn request_settings_for_all_workspaces(&self) {
        if let Some(workspace) = &self.initialize_params.capabilities.workspace
            && workspace.configuration == Some(true)
        {
            self.send_request::<WorkspaceConfiguration>(ConfigurationParams {
                items: self
                    .workspaces
                    .workspaces
                    .read()
                    .keys()
                    .map(|uri| Some(Url::from_file_path(uri).unwrap()))
                    // add default workspace
                    .chain(once(None))
                    .map(|url| ConfigurationItem {
                        scope_uri: url,
                        section: Some(PYTHON_SECTION.to_owned()),
                    })
                    .collect::<Vec<_>>(),
            });
        }
    }

    fn handle_response<'a>(
        &'a self,
        ide_transaction_manager: &mut IDETransactionManager<'a>,
        request: &Request,
        response: &Response,
    ) -> anyhow::Result<()> {
        if let Some((request, response)) =
            as_request_response_pair::<WorkspaceConfiguration>(request, response)
        {
            let mut modified = false;
            for (i, id) in request.items.iter().enumerate() {
                let config: LspConfig = if let Some(value) = response.get(i) {
                    serde_json::from_value(value.clone()).unwrap_or_default()
                } else {
                    continue;
                };
                self.apply_client_configuration(&mut modified, &id.scope_uri, config);
            }
            if modified {
                return self.validate_in_memory(ide_transaction_manager);
            }
        }
        Ok(())
    }

    /// Applies the LSP client configuration to the `scope_uri` (workspace) given.
    ///
    /// The `modified` flag is changed to `true` when the configuration gets applied to the
    /// `scope_uri` matching a valid workspace
    fn apply_client_configuration(
        &self,
        modified: &mut bool,
        scope_uri: &Option<Url>,
        config: LspConfig,
    ) {
        if let Some(python_path) = config.python_path {
            self.update_pythonpath(modified, scope_uri, &python_path);
        }

        if let Some(pyrefly) = config.pyrefly {
            if let Some(extra_paths) = pyrefly.extra_paths {
                self.update_search_paths(modified, scope_uri, extra_paths);
            }
            if let Some(disable_language_services) = pyrefly.disable_language_services {
                self.update_disable_language_services(scope_uri, disable_language_services);
            }
            if let Some(disable_type_errors) = pyrefly.disable_type_errors {
                self.update_disable_type_errors(modified, scope_uri, disable_type_errors);
            }
        }
    }

    /// Update disableLanguageServices setting for scope_uri, None if default workspace
    fn update_disable_language_services(
        &self,
        scope_uri: &Option<Url>,
        disable_language_services: bool,
    ) {
        let mut workspaces = self.workspaces.workspaces.write();

        match scope_uri {
            Some(scope_uri) => {
                if let Some(workspace) = workspaces.get_mut(&scope_uri.to_file_path().unwrap()) {
                    workspace.disable_language_services = disable_language_services;
                }
            }
            None => {
                self.workspaces.default.write().disable_language_services =
                    disable_language_services
            }
        }
    }

    /// Update typeCheckingMode setting for scope_uri, None if default workspace
    fn update_disable_type_errors(
        &self,
        modified: &mut bool,
        scope_uri: &Option<Url>,
        disable_type_errors: bool,
    ) {
        let mut workspaces = self.workspaces.workspaces.write();
        match scope_uri {
            Some(scope_uri) => {
                if let Some(workspace) = workspaces.get_mut(&scope_uri.to_file_path().unwrap()) {
                    *modified = true;
                    workspace.disable_type_errors = disable_type_errors;
                }
            }
            None => {
                *modified = true;
                self.workspaces.default.write().disable_type_errors = disable_type_errors
            }
        }
    }

    fn invalidate_config(&self) {
        self.invalidate(|t| t.invalidate_config());
    }

    /// Updates pythonpath with specified python path
    /// scope_uri = None for default workspace
    fn update_pythonpath(&self, modified: &mut bool, scope_uri: &Option<Url>, python_path: &str) {
        let mut workspaces = self.workspaces.workspaces.write();
        let interpreter = PathBuf::from(python_path);
        let python_info = Some(PythonInfo::new(interpreter));
        match scope_uri {
            Some(scope_uri) => {
                let workspace_path = scope_uri.to_file_path().unwrap();
                if let Some(workspace) = workspaces.get_mut(&workspace_path) {
                    *modified = true;
                    workspace.python_info = python_info;
                }
            }
            None => {
                *modified = true;
                self.workspaces.default.write().python_info = python_info;
            }
        }
        if *modified {
            self.state.increment_snapshot(); // Increment snapshot on python path change
        }
        self.invalidate_config();
    }

    // Updates search paths for scope uri.
    fn update_search_paths(
        &self,
        modified: &mut bool,
        scope_uri: &Option<Url>,
        search_paths: Vec<PathBuf>,
    ) {
        let mut workspaces = self.workspaces.workspaces.write();
        match scope_uri {
            Some(scope_uri) => {
                let workspace_path = scope_uri.to_file_path().unwrap();
                if let Some(workspace) = workspaces.get_mut(&workspace_path) {
                    *modified = true;
                    workspace.search_path = Some(search_paths);
                }
            }
            None => {
                *modified = true;
                self.workspaces.default.write().search_path = Some(search_paths);
            }
        }
        self.invalidate_config();
    }
}

// Helper function to create a default type when we can't determine the actual type
fn create_default_type_for_declaration(decl: &tsp::Declaration) -> tsp::Type {
    let (category, flags) = match decl.category {
        tsp::DeclarationCategory::FUNCTION => (tsp::TypeCategory::FUNCTION, tsp::TypeFlags::new().with_callable()),
        tsp::DeclarationCategory::CLASS => (tsp::TypeCategory::CLASS, tsp::TypeFlags::new().with_instantiable()),
        tsp::DeclarationCategory::IMPORT => (tsp::TypeCategory::MODULE, tsp::TypeFlags::new()),
        tsp::DeclarationCategory::TYPE_ALIAS => (tsp::TypeCategory::ANY, tsp::TypeFlags::new().with_from_alias()),
        tsp::DeclarationCategory::TYPE_PARAM => (tsp::TypeCategory::TYPE_VAR, tsp::TypeFlags::new()),
        _ => (tsp::TypeCategory::ANY, tsp::TypeFlags::new()),
    };

    tsp::Type {
        handle: decl.handle.clone(),
        category,
        flags,
        module_name: Some(decl.module_name.clone()),
        name: decl.name.clone(),
        category_flags: 0,
        decl: None,
    }
}

fn as_notification<T>(x: &Notification) -> Option<T::Params>
where
    T: lsp_types::notification::Notification,
    T::Params: DeserializeOwned,
{
    if x.method == T::METHOD {
        let params = serde_json::from_value(x.params.clone()).unwrap_or_else(|err| {
            panic!(
                "Invalid notification\nMethod: {}\n error: {}",
                x.method, err
            )
        });
        Some(params)
    } else {
        None
    }
}

fn as_request<T>(x: &Request) -> Option<T::Params>
where
    T: lsp_types::request::Request,
    T::Params: DeserializeOwned,
{
    if x.method == T::METHOD {
        let params = serde_json::from_value(x.params.clone()).unwrap_or_else(|err| {
            panic!(
                "Invalid request\n  method: {}\n  error: {}\n  request: {:?}\n",
                x.method, err, x
            )
        });
        Some(params)
    } else {
        None
    }
}

fn as_request_response_pair<T>(
    request: &Request,
    response: &Response,
) -> Option<(T::Params, T::Result)>
where
    T: lsp_types::request::Request,
    T::Params: DeserializeOwned,
{
    if response.id != request.id {
        return None;
    }
    let params = as_request::<T>(request)?;
    let result = serde_json::from_value(response.result.clone()?).unwrap_or_else(|err| {
        panic!(
            "Invalid response\n  method: {}\n response:{:?}\n, response error:{:?}\n, error: {}\n",
            request.method, response.result, response.error, err
        )
    });
    Some((params, result))
}

/// Create a new `Notification` object with the correct name from the given params.
fn new_notification<T>(params: T::Params) -> Notification
where
    T: lsp_types::notification::Notification,
{
    Notification {
        method: T::METHOD.to_owned(),
        params: serde_json::to_value(&params).unwrap(),
    }
}

fn new_response<T>(id: RequestId, params: anyhow::Result<T>) -> Response
where
    T: serde::Serialize,
{
    match params {
        Ok(params) => Response {
            id,
            result: Some(serde_json::to_value(params).unwrap()),
            error: None,
        },
        Err(e) => Response {
            id,
            result: None,
            error: Some(ResponseError {
                code: 0,
                message: format!("{e:#?}"),
                data: None,
            }),
        },
    }
}

fn new_response_with_error_code<T>(id: RequestId, params: Result<T, ResponseError>) -> Response
where
    T: serde::Serialize,
{
    match params {
        Ok(params) => Response {
            id,
            result: Some(serde_json::to_value(params).unwrap()),
            error: None,
        },
        Err(error) => Response {
            id,
            result: None,
            error: Some(error),
        },
    }
}

