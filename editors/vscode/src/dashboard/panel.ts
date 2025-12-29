/**
 * Dashboard Panel
 *
 * Webview panel that displays the compliance dashboard.
 */

import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export class DashboardPanel {
    public static currentPanel: DashboardPanel | undefined;
    public static readonly viewType = 'utf8dokDashboard';

    private readonly panel: vscode.WebviewPanel;
    private readonly extensionUri: vscode.Uri;
    private readonly client: LanguageClient | undefined;
    private disposables: vscode.Disposable[] = [];

    public static createOrShow(
        extensionUri: vscode.Uri,
        client: LanguageClient | undefined
    ): void {
        const column = vscode.window.activeTextEditor
            ? vscode.window.activeTextEditor.viewColumn
            : undefined;

        // If we already have a panel, show it
        if (DashboardPanel.currentPanel) {
            DashboardPanel.currentPanel.panel.reveal(column);
            DashboardPanel.currentPanel.refresh();
            return;
        }

        // Otherwise, create a new panel
        const panel = vscode.window.createWebviewPanel(
            DashboardPanel.viewType,
            'UTF8DOK Compliance Dashboard',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
                localResourceRoots: [
                    vscode.Uri.joinPath(extensionUri, 'media')
                ]
            }
        );

        DashboardPanel.currentPanel = new DashboardPanel(panel, extensionUri, client);
    }

    private constructor(
        panel: vscode.WebviewPanel,
        extensionUri: vscode.Uri,
        client: LanguageClient | undefined
    ) {
        this.panel = panel;
        this.extensionUri = extensionUri;
        this.client = client;

        // Set initial content
        this.refresh();

        // Listen for when the panel is disposed
        this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

        // Handle messages from the webview
        this.panel.webview.onDidReceiveMessage(
            async (message) => {
                switch (message.command) {
                    case 'refresh':
                        await this.refresh();
                        break;
                    case 'openFile':
                        await this.openFile(message.uri);
                        break;
                    case 'runAudit':
                        await vscode.commands.executeCommand('utf8dok.runAudit');
                        break;
                }
            },
            null,
            this.disposables
        );

        // Update when diagnostics change
        vscode.languages.onDidChangeDiagnostics(
            () => this.refresh(),
            null,
            this.disposables
        );
    }

    public async refresh(): Promise<void> {
        const diagnostics = this.collectDiagnostics();
        this.panel.webview.html = this.getHtmlContent(diagnostics);
    }

    private collectDiagnostics(): DiagnosticSummary {
        const allDiagnostics = vscode.languages.getDiagnostics();
        const summary: DiagnosticSummary = {
            total: 0,
            errors: 0,
            warnings: 0,
            info: 0,
            byFile: [],
            byCode: new Map()
        };

        for (const [uri, diagnostics] of allDiagnostics) {
            const utf8dokDiags = diagnostics.filter(d => d.source === 'utf8dok');
            if (utf8dokDiags.length === 0) continue;

            const fileEntry: FileDiagnostics = {
                uri: uri.toString(),
                name: uri.path.split('/').pop() || uri.path,
                diagnostics: []
            };

            for (const diag of utf8dokDiags) {
                summary.total++;

                switch (diag.severity) {
                    case vscode.DiagnosticSeverity.Error:
                        summary.errors++;
                        break;
                    case vscode.DiagnosticSeverity.Warning:
                        summary.warnings++;
                        break;
                    default:
                        summary.info++;
                }

                const code = typeof diag.code === 'object'
                    ? diag.code.value.toString()
                    : diag.code?.toString() || 'UNKNOWN';

                summary.byCode.set(code, (summary.byCode.get(code) || 0) + 1);

                fileEntry.diagnostics.push({
                    message: diag.message,
                    severity: diag.severity,
                    code,
                    line: diag.range.start.line + 1
                });
            }

            summary.byFile.push(fileEntry);
        }

        return summary;
    }

    private getHtmlContent(summary: DiagnosticSummary): string {
        const nonce = this.getNonce();

        const fileList = summary.byFile.map(file => `
            <div class="file-card">
                <div class="file-header" onclick="toggleFile('${this.escapeHtml(file.uri)}')">
                    <span class="file-name">${this.escapeHtml(file.name)}</span>
                    <span class="badge ${file.diagnostics.some(d => d.severity === 0) ? 'error' : 'warning'}">
                        ${file.diagnostics.length}
                    </span>
                </div>
                <div class="file-diagnostics" id="file-${this.hashCode(file.uri)}">
                    ${file.diagnostics.map(d => `
                        <div class="diagnostic ${this.getSeverityClass(d.severity)}"
                             onclick="openFile('${this.escapeHtml(file.uri)}', ${d.line})">
                            <span class="diagnostic-code">${this.escapeHtml(d.code)}</span>
                            <span class="diagnostic-message">${this.escapeHtml(d.message)}</span>
                            <span class="diagnostic-line">:${d.line}</span>
                        </div>
                    `).join('')}
                </div>
            </div>
        `).join('');

        const codeBreakdown = Array.from(summary.byCode.entries())
            .sort((a, b) => b[1] - a[1])
            .map(([code, count]) => `
                <div class="code-stat">
                    <span class="code-name">${this.escapeHtml(code)}</span>
                    <span class="code-count">${count}</span>
                </div>
            `).join('');

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}';">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UTF8DOK Compliance Dashboard</title>
    <style>
        :root {
            --bg-primary: var(--vscode-editor-background);
            --bg-secondary: var(--vscode-sideBar-background);
            --text-primary: var(--vscode-editor-foreground);
            --text-secondary: var(--vscode-descriptionForeground);
            --border-color: var(--vscode-panel-border);
            --error-color: var(--vscode-errorForeground);
            --warning-color: var(--vscode-editorWarning-foreground);
            --info-color: var(--vscode-editorInfo-foreground);
            --success-color: var(--vscode-testing-iconPassed);
        }

        body {
            font-family: var(--vscode-font-family);
            font-size: var(--vscode-font-size);
            color: var(--text-primary);
            background: var(--bg-primary);
            padding: 20px;
            margin: 0;
        }

        h1 {
            font-size: 1.5em;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .header-actions {
            margin-left: auto;
        }

        button {
            background: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border: none;
            padding: 6px 14px;
            cursor: pointer;
            border-radius: 2px;
        }

        button:hover {
            background: var(--vscode-button-hoverBackground);
        }

        .summary {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 15px;
            margin-bottom: 25px;
        }

        .summary-card {
            background: var(--bg-secondary);
            padding: 15px;
            border-radius: 4px;
            border: 1px solid var(--border-color);
        }

        .summary-card.success {
            border-color: var(--success-color);
        }

        .summary-card.error {
            border-color: var(--error-color);
        }

        .summary-card.warning {
            border-color: var(--warning-color);
        }

        .summary-number {
            font-size: 2em;
            font-weight: bold;
        }

        .summary-label {
            color: var(--text-secondary);
            font-size: 0.85em;
        }

        .section-title {
            font-size: 1.1em;
            margin: 20px 0 10px 0;
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 5px;
        }

        .file-card {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            margin-bottom: 10px;
        }

        .file-header {
            display: flex;
            align-items: center;
            padding: 10px 15px;
            cursor: pointer;
        }

        .file-header:hover {
            background: var(--vscode-list-hoverBackground);
        }

        .file-name {
            flex: 1;
            font-weight: 500;
        }

        .badge {
            padding: 2px 8px;
            border-radius: 10px;
            font-size: 0.8em;
        }

        .badge.error {
            background: var(--error-color);
            color: white;
        }

        .badge.warning {
            background: var(--warning-color);
            color: black;
        }

        .file-diagnostics {
            border-top: 1px solid var(--border-color);
        }

        .diagnostic {
            display: flex;
            align-items: center;
            padding: 8px 15px;
            cursor: pointer;
            gap: 10px;
        }

        .diagnostic:hover {
            background: var(--vscode-list-hoverBackground);
        }

        .diagnostic.error {
            border-left: 3px solid var(--error-color);
        }

        .diagnostic.warning {
            border-left: 3px solid var(--warning-color);
        }

        .diagnostic.info {
            border-left: 3px solid var(--info-color);
        }

        .diagnostic-code {
            font-family: var(--vscode-editor-font-family);
            background: var(--vscode-textCodeBlock-background);
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 0.85em;
        }

        .diagnostic-message {
            flex: 1;
            color: var(--text-secondary);
        }

        .diagnostic-line {
            color: var(--text-secondary);
            font-size: 0.85em;
        }

        .code-breakdown {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 10px;
        }

        .code-stat {
            display: flex;
            justify-content: space-between;
            padding: 8px 12px;
            background: var(--bg-secondary);
            border-radius: 4px;
        }

        .code-count {
            font-weight: bold;
        }

        .empty-state {
            text-align: center;
            padding: 40px;
            color: var(--text-secondary);
        }

        .empty-state .icon {
            font-size: 3em;
            margin-bottom: 15px;
        }
    </style>
</head>
<body>
    <h1>
        UTF8DOK Compliance Dashboard
        <div class="header-actions">
            <button onclick="refresh()">Refresh</button>
            <button onclick="runAudit()">Run Audit</button>
        </div>
    </h1>

    ${summary.total === 0 ? `
        <div class="empty-state">
            <div class="icon">&#10003;</div>
            <h2>All Clear!</h2>
            <p>No compliance issues found in the workspace.</p>
        </div>
    ` : `
        <div class="summary">
            <div class="summary-card ${summary.total === 0 ? 'success' : summary.errors > 0 ? 'error' : 'warning'}">
                <div class="summary-number">${summary.total}</div>
                <div class="summary-label">Total Issues</div>
            </div>
            <div class="summary-card ${summary.errors > 0 ? 'error' : ''}">
                <div class="summary-number">${summary.errors}</div>
                <div class="summary-label">Errors</div>
            </div>
            <div class="summary-card ${summary.warnings > 0 ? 'warning' : ''}">
                <div class="summary-number">${summary.warnings}</div>
                <div class="summary-label">Warnings</div>
            </div>
            <div class="summary-card">
                <div class="summary-number">${summary.info}</div>
                <div class="summary-label">Info</div>
            </div>
        </div>

        <h2 class="section-title">Issues by Rule</h2>
        <div class="code-breakdown">
            ${codeBreakdown || '<p class="empty-state">No issues</p>'}
        </div>

        <h2 class="section-title">Issues by File</h2>
        ${fileList || '<p class="empty-state">No issues</p>'}
    `}

    <script nonce="${nonce}">
        const vscode = acquireVsCodeApi();

        function refresh() {
            vscode.postMessage({ command: 'refresh' });
        }

        function runAudit() {
            vscode.postMessage({ command: 'runAudit' });
        }

        function openFile(uri, line) {
            vscode.postMessage({ command: 'openFile', uri, line });
        }

        function toggleFile(uri) {
            // Could implement expand/collapse here
        }
    </script>
</body>
</html>`;
    }

    private getSeverityClass(severity: vscode.DiagnosticSeverity): string {
        switch (severity) {
            case vscode.DiagnosticSeverity.Error:
                return 'error';
            case vscode.DiagnosticSeverity.Warning:
                return 'warning';
            default:
                return 'info';
        }
    }

    private escapeHtml(text: string): string {
        return text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;');
    }

    private hashCode(str: string): string {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return Math.abs(hash).toString(36);
    }

    private getNonce(): string {
        let text = '';
        const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
        for (let i = 0; i < 32; i++) {
            text += possible.charAt(Math.floor(Math.random() * possible.length));
        }
        return text;
    }

    private async openFile(uri: string): Promise<void> {
        const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
        await vscode.window.showTextDocument(document);
    }

    public dispose(): void {
        DashboardPanel.currentPanel = undefined;

        this.panel.dispose();

        while (this.disposables.length) {
            const disposable = this.disposables.pop();
            if (disposable) {
                disposable.dispose();
            }
        }
    }
}

interface DiagnosticSummary {
    total: number;
    errors: number;
    warnings: number;
    info: number;
    byFile: FileDiagnostics[];
    byCode: Map<string, number>;
}

interface FileDiagnostics {
    uri: string;
    name: string;
    diagnostics: DiagnosticInfo[];
}

interface DiagnosticInfo {
    message: string;
    severity: vscode.DiagnosticSeverity;
    code: string;
    line: number;
}
