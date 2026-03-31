//! shx AST — パース結果の中間表現。
//!
//! shx 独自構文（if/for/while/match/function）はそれぞれ専用ノードを持ち、
//! 通常のシェルコードは `Node::Raw` としてそのまま保持される。

/// shx スクリプトの構文ノード。
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Raw shell code passed through unchanged
    Raw(String),
    /// Shell comment (e.g. # this is a comment)
    Comment(String),
    /// if/elif/else with brace syntax
    If {
        branches: Vec<Branch>,
        else_body: Option<Vec<Node>>,
    },
    /// for loop with brace syntax
    For {
        var: String,
        list: String,
        body: Vec<Node>,
    },
    /// while loop with brace syntax
    While {
        condition: String,
        body: Vec<Node>,
    },
    /// match expression (transpiles to case/esac)
    Match {
        expr: String,
        arms: Vec<MatchArm>,
    },
    /// Function definition: name() { body }
    Function {
        name: String,
        body: Vec<Node>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub condition: String,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: String,
    pub body: Vec<Node>,
}
