//! Redline table - frozen rules that the judge gate enforces.

use super::types::RedlineRule;

/// The frozen redline table v1.
/// These四条 rules are non-negotiable invariants enforced by the judge gate.
pub const REDLINE_TABLE: [RedlineRule; 4] = [
    RedlineRule {
        id: "I-NEG-1",
        statement: "用户数据文件（配置、会话、记忆、episodes）在其原路径保持可访问且内容非空——固化动作不得导致其消失、移位或清空。",
    },
    RedlineRule {
        id: "I-NEG-2",
        statement: "未过门的固化产物不得出现在 agent 运行时可自动命中的位置（技能 loader 扫描位、prompt 注入面、配置读取面）。",
    },
    RedlineRule {
        id: "I-NEG-3",
        statement: "红线表与门禁执行代码不被固化动作自身修改（门禁不得批准改写自己）。",
    },
    RedlineRule {
        id: "I-NEG-4",
        statement: "审计日志只可追加——固化动作不得删除、截断或改写历史审计记录。",
    },
];

/// Returns the ordered list of redline rule IDs.
pub fn redline_ids() -> [&'static str; 4] {
    ["I-NEG-1", "I-NEG-2", "I-NEG-3", "I-NEG-4"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redline_table_length_is_four() {
        assert_eq!(REDLINE_TABLE.len(), 4);
    }

    #[test]
    fn redline_ids_ordered_correctly() {
        let ids = redline_ids();
        assert_eq!(ids, ["I-NEG-1", "I-NEG-2", "I-NEG-3", "I-NEG-4"]);
    }

    #[test]
    fn redline_statements_all_non_empty() {
        for rule in &REDLINE_TABLE {
            assert!(!rule.statement.is_empty(), "Rule {} has empty statement", rule.id);
        }
    }
}
