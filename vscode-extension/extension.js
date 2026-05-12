const vscode = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;

/**
 * Activate the Matter extension
 */
function activate(context) {
    console.log('Matter extension is now active');

    // Get configuration
    const config = vscode.workspace.getConfiguration('matter');
    const lspEnabled = config.get('lsp.enabled', true);
    const matterCliPath = config.get('lsp.path', 'matter-cli');

    // Start LSP client if enabled
    if (lspEnabled) {
        startLanguageClient(context, matterCliPath);
    }

    // Register commands
    registerCommands(context, matterCliPath);

    // Register formatters
    registerFormatters(context, matterCliPath);
}

/**
 * Start the Language Server Protocol client
 */
function startLanguageClient(context, matterCliPath) {
    const serverOptions = {
        command: matterCliPath,
        args: ['lsp'],
        transport: TransportKind.stdio
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'matter' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.matter')
        }
    };

    client = new LanguageClient(
        'matterLanguageServer',
        'Matter Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
    context.subscriptions.push(client);

    console.log('Matter LSP client started');
}

/**
 * Register extension commands
 */
function registerCommands(context, matterCliPath) {
    // Run File
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

    // Compile File
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

    // Run Bytecode
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

    // Format File
    context.subscriptions.push(
        vscode.commands.registerCommand('matter.formatFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }

            const filePath = editor.document.uri.fsPath;
            await runCommand(matterCliPath, ['format', filePath, '--write'], 'Formatting Matter file...');
            
            // Reload the file
            await vscode.commands.executeCommand('workbench.action.files.revert');
        })
    );

    // Lint File
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

    // Debug File
    context.subscriptions.push(
        vscode.commands.registerCommand('matter.debugFile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active Matter file');
                return;
            }

            const filePath = editor.document.uri.fsPath;
            
            // Open integrated terminal and run debugger
            const terminal = vscode.window.createTerminal('Matter Debugger');
            terminal.show();
            terminal.sendText(`${matterCliPath} debug "${filePath}"`);
        })
    );

    // Show Backends
    context.subscriptions.push(
        vscode.commands.registerCommand('matter.showBackends', async () => {
            await runCommand(matterCliPath, ['backends'], 'Available backends...');
        })
    );

    // Show Examples
    context.subscriptions.push(
        vscode.commands.registerCommand('matter.showExamples', async () => {
            await runCommand(matterCliPath, ['examples'], 'Available examples...');
        })
    );
}

/**
 * Register document formatters
 */
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

                    // Run formatter
                    await execAsync(`${matterCliPath} format "${filePath}" --write`);

                    // Read formatted content
                    const fs = require('fs');
                    const formattedContent = fs.readFileSync(filePath, 'utf8');

                    // Return edit that replaces entire document
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

/**
 * Run a command and show output
 */
async function runCommand(command, args, message) {
    const { exec } = require('child_process');
    const { promisify } = require('util');
    const execAsync = promisify(exec);

    try {
        vscode.window.showInformationMessage(message);
        
        const fullCommand = `${command} ${args.map(a => `"${a}"`).join(' ')}`;
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

/**
 * Deactivate the extension
 */
function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = {
    activate,
    deactivate
};
