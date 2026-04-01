//! shx LSP サーバー — diagnostics とフォーマットを提供する。

use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

pub struct Backend {
    client: Client,
    /// ドキュメントのテキストをキャッシュ（formatting で使用）
    documents: Mutex<HashMap<Url, String>>,
}

impl Backend {
    fn validate(&self, uri: Url, text: &str) {
        // キャッシュを更新
        if let Ok(mut docs) = self.documents.lock() {
            docs.insert(uri.clone(), text.to_string());
        }

        let tokens = crate::lexer::tokenize(text);
        let diagnostics = match crate::parser::parse(tokens) {
            Ok(_) => vec![],
            Err(e) => {
                let line = e.span.line.saturating_sub(1) as u32;
                let col = e.span.column.saturating_sub(1) as u32;
                // エラー位置から行末までを範囲にする
                let end_col = text.lines().nth(line as usize)
                    .map(|l| l.len() as u32)
                    .unwrap_or(col + 1);
                vec![Diagnostic {
                    range: Range {
                        start: Position { line, character: col },
                        end: Position { line, character: end_col },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("shx".into()),
                    message: e.message,
                    ..Default::default()
                }]
            }
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            client.publish_diagnostics(uri, diagnostics, None).await;
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "shx LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate(
            params.text_document.uri,
            &params.text_document.text,
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.validate(params.text_document.uri, &change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Ok(mut docs) = self.documents.lock() {
            docs.remove(&params.text_document.uri);
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let text = {
            let docs = self.documents.lock().unwrap();
            match docs.get(&params.text_document.uri) {
                Some(t) => t.clone(),
                None => return Ok(None),
            }
        };

        let formatted = match crate::format_source(&text) {
            Ok(f) => f,
            Err(_) => return Ok(None), // パースエラー時はフォーマットしない
        };

        // ドキュメント全体を置換
        let line_count = text.lines().count() as u32;
        let last_line_len = text.lines().last().map(|l| l.len() as u32).unwrap_or(0);
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: line_count, character: last_line_len },
            },
            new_text: formatted,
        }]))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// LSP サーバーを起動する。stdin/stdout で通信する。
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
