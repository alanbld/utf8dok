/**
 * Status Bar Manager
 *
 * Shows UTF8DOK compliance status in the VS Code status bar.
 */

import * as vscode from 'vscode';

export class StatusBarManager implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private issueCount: number = 0;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'utf8dok.showDashboard';
        this.statusBarItem.show();
        this.setLoading('Initializing...');
    }

    setLoading(message: string): void {
        this.statusBarItem.text = `$(sync~spin) UTF8DOK: ${message}`;
        this.statusBarItem.tooltip = 'UTF8DOK is starting...';
        this.statusBarItem.backgroundColor = undefined;
    }

    setReady(): void {
        this.updateStatus();
    }

    setError(message: string): void {
        this.statusBarItem.text = `$(error) UTF8DOK: ${message}`;
        this.statusBarItem.tooltip = 'Click to see details';
        this.statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.errorBackground'
        );
    }

    updateIssueCount(count: number): void {
        this.issueCount = count;
        this.updateStatus();
    }

    private updateStatus(): void {
        if (this.issueCount === 0) {
            this.statusBarItem.text = '$(check) UTF8DOK';
            this.statusBarItem.tooltip = 'All compliance checks passed';
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.statusBarItem.text = `$(warning) UTF8DOK: ${this.issueCount} issue${this.issueCount > 1 ? 's' : ''}`;
            this.statusBarItem.tooltip = `${this.issueCount} compliance issue${this.issueCount > 1 ? 's' : ''} found. Click to open dashboard.`;
            this.statusBarItem.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.warningBackground'
            );
        }
    }

    dispose(): void {
        this.statusBarItem.dispose();
    }
}
