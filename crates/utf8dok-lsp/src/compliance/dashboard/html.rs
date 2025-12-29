//! HTML Dashboard Generator
//!
//! Generates a self-contained HTML dashboard for compliance reporting.
//! Designed for management visibility and web-based audit reviews.

use super::timestamp;
use crate::compliance::{ComplianceResult, ViolationSeverity};

/// HTML report generator
pub struct HtmlGenerator;

impl HtmlGenerator {
    /// Generate a complete HTML dashboard
    #[allow(dead_code)]
    pub fn generate(result: &ComplianceResult, rules: Vec<(&str, &str)>) -> String {
        let violations_html = Self::format_violations(result);
        let rules_html = Self::format_rules(&rules);

        let status_color = if result.errors > 0 {
            "#dc3545"
        } else if result.warnings > 0 {
            "#ffc107"
        } else {
            "#28a745"
        };

        let status_text = if result.errors > 0 {
            "FAILING"
        } else if result.warnings > 0 {
            "PASSING (with warnings)"
        } else {
            "PASSING"
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UTF8DOK Compliance Dashboard</title>
    <style>
        * {{ box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
            background: #f8f9fa;
        }}
        h1 {{
            color: #212529;
            border-bottom: 2px solid #dee2e6;
            padding-bottom: 0.5rem;
        }}
        h2 {{ color: #495057; margin-top: 2rem; }}
        .dashboard {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }}
        .stat-card {{
            background: white;
            padding: 1.5rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            text-align: center;
        }}
        .stat-value {{
            font-size: 2.5rem;
            font-weight: bold;
            margin-bottom: 0.25rem;
        }}
        .stat-label {{
            color: #6c757d;
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}
        .status-badge {{
            display: inline-block;
            padding: 0.5rem 1rem;
            border-radius: 20px;
            font-weight: bold;
            font-size: 0.9rem;
        }}
        .violations-container {{
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .violation {{
            padding: 1rem;
            border-left: 4px solid;
            margin: 0.75rem 0;
            border-radius: 0 4px 4px 0;
        }}
        .violation-error {{
            border-left-color: #dc3545;
            background: #fff5f5;
        }}
        .violation-warning {{
            border-left-color: #ffc107;
            background: #fffbeb;
        }}
        .violation-info {{
            border-left-color: #17a2b8;
            background: #f0f9ff;
        }}
        .violation-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }}
        .violation-code {{
            font-family: 'SF Mono', Monaco, 'Courier New', monospace;
            font-weight: bold;
        }}
        .severity-badge {{
            display: inline-block;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            font-size: 0.75rem;
            font-weight: bold;
            text-transform: uppercase;
        }}
        .severity-error {{ background: #dc3545; color: white; }}
        .severity-warning {{ background: #ffc107; color: #212529; }}
        .severity-info {{ background: #17a2b8; color: white; }}
        .violation-message {{ color: #495057; }}
        .violation-location {{
            font-family: 'SF Mono', Monaco, 'Courier New', monospace;
            font-size: 0.85rem;
            color: #6c757d;
            margin-top: 0.5rem;
        }}
        .success-message {{
            padding: 2rem;
            text-align: center;
            color: #28a745;
            font-size: 1.25rem;
        }}
        .rules-list {{
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .rule {{
            padding: 0.75rem 0;
            border-bottom: 1px solid #dee2e6;
        }}
        .rule:last-child {{ border-bottom: none; }}
        .rule-code {{
            font-family: 'SF Mono', Monaco, 'Courier New', monospace;
            font-weight: bold;
            color: #495057;
        }}
        .rule-description {{ color: #6c757d; margin-left: 1rem; }}
        footer {{
            margin-top: 3rem;
            padding-top: 1rem;
            border-top: 1px solid #dee2e6;
            color: #6c757d;
            font-size: 0.85rem;
            text-align: center;
        }}
        footer a {{ color: #007bff; text-decoration: none; }}
        footer a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <h1>UTF8DOK Compliance Dashboard</h1>

    <div class="dashboard">
        <div class="stat-card">
            <div class="stat-value">{total_documents}</div>
            <div class="stat-label">Documents</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" style="color: #dc3545;">{errors}</div>
            <div class="stat-label">Errors</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" style="color: #ffc107;">{warnings}</div>
            <div class="stat-label">Warnings</div>
        </div>
        <div class="stat-card">
            <div class="stat-value" style="color: {status_color};">{compliance_score}%</div>
            <div class="stat-label">Compliance Score</div>
        </div>
    </div>

    <div style="text-align: center; margin-bottom: 2rem;">
        <span class="status-badge" style="background: {status_color}; color: white;">
            {status_text}
        </span>
    </div>

    <h2>Violations</h2>
    <div class="violations-container">
        {violations_html}
    </div>

    <h2>Active Rules</h2>
    <div class="rules-list">
        {rules_html}
    </div>

    <footer>
        Generated by <a href="https://github.com/utf8dok/utf8dok">UTF8DOK Compliance Engine</a> at {timestamp}
    </footer>
</body>
</html>"#,
            total_documents = result.total_documents,
            errors = result.errors,
            warnings = result.warnings,
            compliance_score = result.compliance_score,
            status_color = status_color,
            status_text = status_text,
            violations_html = violations_html,
            rules_html = rules_html,
            timestamp = timestamp()
        )
    }

    fn format_violations(result: &ComplianceResult) -> String {
        if result.violations.is_empty() {
            return r#"<div class="success-message">All compliance checks passed!</div>"#
                .to_string();
        }

        let mut html = String::new();

        for violation in &result.violations {
            let (severity_class, severity_badge) = match violation.severity {
                ViolationSeverity::Error => (
                    "violation-error",
                    r#"<span class="severity-badge severity-error">ERROR</span>"#,
                ),
                ViolationSeverity::Warning => (
                    "violation-warning",
                    r#"<span class="severity-badge severity-warning">WARNING</span>"#,
                ),
                ViolationSeverity::Info => (
                    "violation-info",
                    r#"<span class="severity-badge severity-info">INFO</span>"#,
                ),
            };

            // Extract filename from URI
            let filename = violation
                .uri
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .unwrap_or("unknown");

            html.push_str(&format!(
                r#"
        <div class="violation {severity_class}">
            <div class="violation-header">
                <span class="violation-code">{code}</span>
                {severity_badge}
            </div>
            <div class="violation-message">{message}</div>
            <div class="violation-location">{filename}:{line}:{col}</div>
        </div>"#,
                severity_class = severity_class,
                code = violation.code,
                severity_badge = severity_badge,
                message = html_escape(&violation.message),
                filename = filename,
                line = violation.range.start.line + 1,
                col = violation.range.start.character + 1
            ));
        }

        html
    }

    fn format_rules(rules: &[(&str, &str)]) -> String {
        if rules.is_empty() {
            return "<div>No rules configured.</div>".to_string();
        }

        let mut html = String::new();

        for (code, description) in rules {
            html.push_str(&format!(
                r#"
        <div class="rule">
            <span class="rule-code">{code}</span>
            <span class="rule-description">{description}</span>
        </div>"#,
                code = code,
                description = html_escape(description)
            ));
        }

        html
    }
}

/// Basic HTML escaping for security
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compliance::Violation;
    use tower_lsp::lsp_types::{Position, Range, Url};

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn test_generate_clean_report() {
        let result = ComplianceResult {
            violations: vec![],
            errors: 0,
            warnings: 0,
            info: 0,
            total_documents: 5,
            compliance_score: 100,
        };

        let html = HtmlGenerator::generate(&result, vec![]);
        assert!(html.contains("All compliance checks passed!"));
        assert!(html.contains("100%"));
        assert!(html.contains("PASSING"));
    }

    #[test]
    fn test_generate_with_violations() {
        let result = ComplianceResult {
            violations: vec![Violation {
                uri: Url::parse("file:///test.adoc").unwrap(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                message: "Test violation".to_string(),
                severity: ViolationSeverity::Error,
                code: "TEST001".to_string(),
            }],
            errors: 1,
            warnings: 0,
            info: 0,
            total_documents: 5,
            compliance_score: 80,
        };

        let html = HtmlGenerator::generate(&result, vec![("TEST001", "Test rule")]);
        assert!(html.contains("TEST001"));
        assert!(html.contains("Test violation"));
        assert!(html.contains("FAILING"));
        assert!(html.contains("Test rule"));
    }
}
