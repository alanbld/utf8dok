/**
 * UTF8DOK VS Code Extension
 *
 * Enterprise documentation compliance and quality platform for AsciiDoc.
 * Provides real-time validation, compliance checking, and writing quality analysis.
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';
import { DashboardPanel } from './dashboard/panel';
import { StatusBarManager } from './status/statusBar';

let client: LanguageClient | undefined;
let statusBar: StatusBarManager | undefined;
let outputChannel: vscode.OutputChannel;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    outputChannel = vscode.window.createOutputChannel('UTF8DOK');
    outputChannel.appendLine('UTF8DOK extension activating...');

    // Initialize status bar
    statusBar = new StatusBarManager();
    context.subscriptions.push(statusBar);

    // Start language server
    try {
        await startLanguageServer(context);
        statusBar.setReady();
    } catch (error) {
        outputChannel.appendLine(`Failed to start language server: ${error}`);
        statusBar.setError('Server failed to start');
        vscode.window.showErrorMessage(
            `UTF8DOK: Failed to start language server. ${error}`
        );
    }

    // Register commands
    registerCommands(context);

    outputChannel.appendLine('UTF8DOK extension activated');
}

async function startLanguageServer(context: vscode.ExtensionContext): Promise<void> {
    const serverPath = await getServerPath(context);
    outputChannel.appendLine(`Using server at: ${serverPath}`);

    if (!fs.existsSync(serverPath)) {
        throw new Error(`Server binary not found at ${serverPath}`);
    }

    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
            options: {
                env: { ...process.env, RUST_LOG: 'debug' }
            }
        }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'asciidoc' },
            { scheme: 'file', pattern: '**/*.adoc' },
            { scheme: 'file', pattern: '**/*.asciidoc' }
        ],
        synchronize: {
            configurationSection: 'utf8dok',
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{adoc,asciidoc,asc}')
        },
        outputChannel: outputChannel,
        initializationOptions: getInitializationOptions()
    };

    client = new LanguageClient(
        'utf8dok',
        'UTF8DOK Language Server',
        serverOptions,
        clientOptions
    );

    // Handle diagnostics for status bar updates
    client.onDidChangeState((event) => {
        if (event.newState === 2) { // Running
            statusBar?.setReady();
        } else if (event.newState === 1) { // Starting
            statusBar?.setLoading('Starting...');
        }
    });

    await client.start();
}

async function getServerPath(context: vscode.ExtensionContext): Promise<string> {
    const config = vscode.workspace.getConfiguration('utf8dok');
    const userPath = config.get<string>('serverPath');

    // User-specified path takes priority
    if (userPath && userPath.trim() !== '') {
        return userPath;
    }

    // Try bundled binary
    const platform = process.platform;
    const arch = process.arch;

    let binaryName = 'utf8dok-lsp';
    if (platform === 'win32') {
        binaryName += '.exe';
    }

    // Platform-specific binary directory
    let platformDir: string;
    if (platform === 'win32') {
        platformDir = 'win32-x64';
    } else if (platform === 'darwin') {
        platformDir = arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64';
    } else {
        platformDir = 'linux-x64';
    }

    const bundledPath = path.join(context.extensionPath, 'bin', platformDir, binaryName);

    if (fs.existsSync(bundledPath)) {
        return bundledPath;
    }

    // Fallback: try to find in PATH or development location
    const devPath = path.join(context.extensionPath, '..', '..', 'target', 'release', binaryName);
    if (fs.existsSync(devPath)) {
        return devPath;
    }

    // Last resort: hope it's in PATH
    return binaryName;
}

function getInitializationOptions(): object {
    const config = vscode.workspace.getConfiguration('utf8dok');

    return {
        compliance: {
            bridge: {
                orphans: config.get('compliance.orphans', 'warning'),
                superseded_status: config.get('compliance.supersededStatus', 'error')
            }
        },
        plugins: {
            diagrams: config.get('plugins.diagrams', true),
            writing_quality: config.get('plugins.writingQuality', true),
            custom_weasel_words: config.get('plugins.customWeaselWords', [])
        },
        workspace: {
            entry_points: config.get('workspace.entryPoints', ['index.adoc', 'README.adoc'])
        }
    };
}

function registerCommands(context: vscode.ExtensionContext): void {
    // Show Dashboard
    context.subscriptions.push(
        vscode.commands.registerCommand('utf8dok.showDashboard', () => {
            DashboardPanel.createOrShow(context.extensionUri, client);
        })
    );

    // Run Audit
    context.subscriptions.push(
        vscode.commands.registerCommand('utf8dok.runAudit', async () => {
            await runComplianceAudit();
        })
    );

    // Fix All
    context.subscriptions.push(
        vscode.commands.registerCommand('utf8dok.fixAll', async () => {
            await fixAllIssues();
        })
    );

    // Restart Server
    context.subscriptions.push(
        vscode.commands.registerCommand('utf8dok.restartServer', async () => {
            await restartServer(context);
        })
    );
}

async function runComplianceAudit(): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    if (!client) {
        vscode.window.showErrorMessage('Language server not running');
        return;
    }

    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: 'Running compliance audit...',
            cancellable: false
        },
        async (progress) => {
            progress.report({ increment: 0, message: 'Scanning documents...' });

            try {
                // Request audit from LSP (this would need a custom LSP command)
                const result = await client!.sendRequest('workspace/executeCommand', {
                    command: 'utf8dok.audit',
                    arguments: [workspaceFolders[0].uri.fsPath]
                });

                progress.report({ increment: 100, message: 'Complete!' });

                // Show results
                const resultStr = result as string;
                const doc = await vscode.workspace.openTextDocument({
                    content: resultStr,
                    language: 'markdown'
                });
                await vscode.window.showTextDocument(doc, { preview: true });

            } catch (error) {
                vscode.window.showErrorMessage(`Audit failed: ${error}`);
            }
        }
    );
}

async function fixAllIssues(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showInformationMessage('No active editor');
        return;
    }

    if (!client) {
        vscode.window.showErrorMessage('Language server not running');
        return;
    }

    // Get diagnostics for the current document
    const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
    const fixableDiagnostics = diagnostics.filter(d => d.source === 'utf8dok');

    if (fixableDiagnostics.length === 0) {
        vscode.window.showInformationMessage('No issues to fix');
        return;
    }

    // Request code actions for each diagnostic
    let fixCount = 0;
    for (const diagnostic of fixableDiagnostics) {
        const codeActions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
            'vscode.executeCodeActionProvider',
            editor.document.uri,
            diagnostic.range
        );

        if (codeActions && codeActions.length > 0) {
            // Apply the first (preferred) fix
            const action = codeActions[0];
            if (action.edit) {
                await vscode.workspace.applyEdit(action.edit);
                fixCount++;
            }
        }
    }

    vscode.window.showInformationMessage(`Fixed ${fixCount} issue(s)`);
}

async function restartServer(context: vscode.ExtensionContext): Promise<void> {
    statusBar?.setLoading('Restarting...');

    if (client) {
        await client.stop();
    }

    try {
        await startLanguageServer(context);
        vscode.window.showInformationMessage('UTF8DOK server restarted');
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to restart server: ${error}`);
    }
}

export function deactivate(): Thenable<void> | undefined {
    outputChannel.appendLine('UTF8DOK extension deactivating...');

    if (!client) {
        return undefined;
    }

    return client.stop();
}
