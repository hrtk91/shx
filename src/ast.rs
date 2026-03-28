#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Raw shell code passed through unchanged
    Raw(String),
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
