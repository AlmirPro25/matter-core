const vscode = require('vscode');
const path = require('path');
const fs = require('fs');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;

/**
 * Activate the Matter extension
 */
function activate(context) {
    console.log('Matter extension is now active');

    const config = vscode.workspace.getConfiguration('matter');
    const lspEnabled = config.get('lsp.enabled', true);
    // Separate from CLI path: LSP is a dedicated binary (matter-lsp / matter-lsp.exe).
    const configuredLspPath = (config.get('lsp.path', '') || '').trim();
    const matterCliPath = (config.get('cli.path', '') || config.get('lsp.cliPath', 'matter-cli') || 'matter-cli').trim() || 'matter-cli';

    if (lspEnabled) {
        startLanguageClient(context, configuredLspPath);
    }

    registerCommands(context, matterCliPath);
    registerFormatters(context, matterCliPath);
}

/**
 * Resolve path to the Matter LSP server binary (not matter-cli).
 * Order:
 *  1) Explicit matter.lsp.path (non-empty)
 *  2) matter-lsp / matter-lsp.exe on PATH
 *  3) Adjacent to matter-cli on PATH (same directory)
 *  4) Relative install / package layout: <ext>/../../bin/matter-lsp.exe (dev monorepo)
 *  5) $LOCALAPPDATA/Matter/bin/matter-lsp.exe (Windows install default)
 * Never defaults to a hard-coded personal drive path.
 */
function resolveMatterLspPath(configured) {
    const candidates = [];

    if (configured) {
        candidates.push(configured);
    }

    // PATH names
    for (const name of ['matter-lsp.exe', 'matter-lsp']) {
        candidates.push(name);
    }

    // Same directory as matter-cli if on PATH
    try {
        const { execSync } = require('child_process');
        for (const cliName of ['matter-cli.exe', 'matter-cli', 'matter.exe', 'matter']) {
            try {
                const which = process.platform === 'win32'
                    ? execSync(`where ${cliName}`, { encoding: 'utf8' }).split(/\r?\n/)[0].trim()
                    : execSync(`command -v ${cliName}`, { encoding: 'utf8' }).trim();
                if (which) {
                    const dir = path.dirname(which);
                    candidates.push(path.join(dir, process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp'));
                }
            } catch (_) { /* not found */ }
        }
    } catch (_) { /* ignore */ }

    // Extension-relative (when packaged next to bin/ or in monorepo)
    const extRoot = path.join(__dirname);
    candidates.push(
        path.join(extRoot, '..', 'bin', process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp'),
        path.join(extRoot, '..', '..', 'bin', process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp'),
        path.join(extRoot, '..', 'target', 'release', process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp'),
        path.join(extRoot, '..', 'target', 'x86_64-pc-windows-gnu', 'release', process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp')
    );

    // Default user install layout (no hard-coded drive letter)
    if (process.env.LOCALAPPDATA) {
        candidates.push(path.join(process.env.LOCALAPPDATA, 'Matter', 'bin', 'matter-lsp.exe'));
    }
    if (process.env.MATTER_HOME) {
        candidates.push(path.join(process.env.MATTER_HOME, 'bin', process.platform === 'win32' ? 'matter-lsp.exe' : 'matter-lsp'));
    }
    if (process.env.MATTER_LSP && process.env.MATTER_LSP.trim()) {
        candidates.unshift(process.env.MATTER_LSP.trim());
    }

    for (const c of candidates) {
        if (!c) continue;
        // bare command name — let LanguageClient resolve via PATH
        if (!c.includes(path.sep) && !c.includes('/') && !c.includes('\\') && !path.isAbsolute(c)) {
            // Prefer absolute when file exists via PATH resolution later; keep as fallback
            try {
                const { execSync } = require('child_process');
                const cmd = process.platform === 'win32' ? `where ${c}` : `command -v ${c}`;
                const found = execSync(cmd, { encoding: 'utf8' }).split(/\r?\n/)[0].trim();
                if (found && fs.existsSync(found)) {
                    return found;
                }
            } catch (_) { /* continue */ }
            continue;
        }
        try {
            if (fs.existsSync(c)) {
                return c;
            }
        } catch (_) { /* continue */ }
    }

    return null;
}

/**
 * Start the Language Server Protocol client against matter-lsp.exe (stdio).
 */
function startLanguageClient(context, configuredLspPath) {
    const serverCommand = resolveMatterLspPath(configuredLspPath);

    if (!serverCommand) {
        const msg =
            'Matter LSP server not found. Install Matter Core (bin/matter-lsp.exe) or set ' +
            'matter.lsp.path to the full path of matter-lsp.exe. ' +
            'Note: language-only matter-cli does not implement "lsp" — use the dedicated binary.';
        vscode.window.showErrorMessage(msg);
        console.error(msg);
        return;
    }

    const serverOptions = {
        command: serverCommand,
        args: [],
        transport: TransportKind.stdio
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'matter' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.matter')
        },
        outputChannelName: 'Matter LSP'
    };

    client = new LanguageClient(
        'matterLanguageServer',
        'Matter Language Server',
        serverOptions,
        clientOptions
    );

    client.start().then(() => {
        console.log('Matter LSP client started:', serverCommand);
    }).catch((err) => {
        vscode.window.showErrorMessage(
            `Failed to start Matter LSP (${serverCommand}): ${err.message || err}`
        );
    });
    context.subscriptions.push(client);
}

/**
 * Register extension commands (CLI-oriented; separate from LSP binary).
 */
function registerCommands(context, matterCliPath) {
    context.subscriptions.push(
        vscode.commands.registerCommand('matter.runFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            await runCommand(matterCliPath, ['run', filePath], 'Running Matter file...');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.compileFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            const outputPath = filePath.replace('.matter', '.mbc');
            await runCommand(matterCliPath, ['compile', filePath, '-o', outputPath], 'Compiling Matter file...');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.runBytecode', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            if (!filePath.endsWith('.mbc')) {
                vscode.window.showErrorMessage('Not a bytecode file (.mbc)');
                return;
            }
            await runCommand(matterCliPath, ['run-bytecode', filePath], 'Running bytecode...');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.formatFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            await runCommand(matterCliPath, ['format', filePath, '--write'], 'Formatting Matter file...');
            await vscode.commands.executeCommand('workbench.action.files.revert');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.lintFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            await runCommand(matterCliPath, ['lint', filePath], 'Linting Matter file...');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.debugFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            const terminal = vscode.window.createTerminal('Matter Debugger');
            terminal.show();
            terminal.sendText(`${matterCliPath} debug "${filePath}"`);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.showBackends', async () => {
            await runCommand(matterCliPath, ['backends'], 'Available backends...');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('matter.showExamples', async () => {
            await runCommand(matterCliPath, ['examples'], 'Available examples...');
        })
    );
}

function registerFormatters(context, matterCliPath) {
    const config = vscode.workspace.getConfiguration('matter');
    const formatterEnabled = config.get('formatter.enabled', true);
    if (!formatterEnabled) {
        return;
    }

    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider('matter', {
            async provideDocumentFormattingEdits(document) {
                const filePath = document.uri.fsPath;
                try {
                    const { exec } = require('child_process');
                    const { promisify } = require('util');
                    const execAsync = promisify(exec);
                    await execAsync(`"${matterCliPath}" format "${filePath}" --write`);
                    const formattedContent = fs.readFileSync(filePath, 'utf8');
                    const firstLine = document.lineAt(0);
                    const lastLine = document.lineAt(document.lineCount - 1);
                    const range = new vscode.Range(firstLine.range.start, lastLine.range.end);
                    return [vscode.TextEdit.replace(range, formattedContent)];
                } catch (error) {
                    vscode.window.showErrorMessage(`Formatting failed: ${error.message}`);
                    return [];
                }
            }
        })
    );
}

async function runCommand(command, args, message) {
    const { exec } = require('child_process');
    const { promisify } = require('util');
    const execAsync = promisify(exec);

    try {
        vscode.window.showInformationMessage(message);
        const fullCommand = `"${command}" ${args.map(a => `"${a}"`).join(' ')}`;
        const { stdout, stderr } = await execAsync(fullCommand);
        if (stdout) {
            const outputChannel = vscode.window.createOutputChannel('Matter');
            outputChannel.clear();
            outputChannel.appendLine(stdout);
            outputChannel.show();
        }
        if (stderr) {
            vscode.window.showErrorMessage(stderr);
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Command failed: ${error.message}`);
    }
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = {
    activate,
    deactivate,
    // Exported for tests / smoke
    resolveMatterLspPath
};
