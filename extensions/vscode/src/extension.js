const vscode = require('vscode');
const { execSync, spawn } = require('child_process');

function activate(context) {
    console.log('Crush extension activated');

    const provider = new CrushContainerProvider();
    vscode.window.registerTreeDataProvider('crush-containers', provider);

    context.subscriptions.push(
        vscode.commands.registerCommand('crush.run', () => runCrush()),
        vscode.commands.registerCommand('crush.debug', () => debugCrush()),
        vscode.commands.registerCommand('crush.ps', () => listContainers()),
        vscode.commands.registerCommand('crush.logs', () => showLogs()),
        vscode.commands.registerCommand('crush.refresh', () => provider.refresh())
    );

    const statusBar = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right, 100
    );
    statusBar.text = "$(package) Crush";
    statusBar.command = 'crush.ps';
    statusBar.show();
    context.subscriptions.push(statusBar);
}
exports.activate = activate;

function runCrush() {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) { return; }
    const root = workspaceFolders[0].uri.fsPath;
    const terminal = vscode.window.createTerminal('Crush');
    terminal.show();
    terminal.sendText(`cd "${root}" && crush`);
}

function debugCrush() {
    const config = vscode.workspace.getConfiguration('crush');
    const containerId = config.get('defaultContainer', '');
    if (!containerId) {
        vscode.window.showInputBox({ prompt: 'Container ID' }).then(id => {
            if (id) debugContainer(id);
        });
    } else {
        debugContainer(containerId);
    }
}

function debugContainer(id) {
    const terminal = vscode.window.createTerminal('Crush Debug');
    terminal.show();
    terminal.sendText(`crush debug ${id}`);
}

function listContainers() {
    const output = vscode.window.createOutputChannel('Crush');
    try {
        const result = execSync('crush ps --format json 2>/dev/null || echo "[]"');
        const containers = JSON.parse(result.toString());
        output.clear();
        output.appendLine('Containers:');
        for (const c of containers) {
            output.appendLine(`  ${c.Id?.slice(0, 12)}  ${c.Image}  ${c.Status}`);
        }
        output.show();
    } catch (e) {
        output.appendLine('Crush not found or not running');
        output.show();
    }
}

function showLogs() {
    vscode.window.showInputBox({ prompt: 'Container ID for logs' }).then(id => {
        if (!id) return;
        const output = vscode.window.createOutputChannel(`Crush: ${id}`);
        const child = spawn('crush', ['logs', '--tail', '50', id]);
        child.stdout.on('data', data => output.append(data.toString()));
        child.stderr.on('data', data => output.append(data.toString()));
        output.show();
    });
}

class CrushContainerProvider {
    constructor() { this._onDidChangeTreeData = new vscode.EventEmitter(); }
    get onDidChangeTreeData() { return this._onDidChangeTreeData.event; }
    refresh() { this._onDidChangeTreeData.fire(undefined); }

    getTreeItem(element) { return element; }

    getChildren(element) {
        if (!element) return this.getContainers();
        return [];
    }

    async getContainers() {
        try {
            const result = execSync('crush ps --format json 2>/dev/null || echo "[]"');
            const containers = JSON.parse(result.toString());
            return containers.map(c => new vscode.TreeItem(
                `${c.Id?.slice(0, 12) || '?'}  ${c.Image || '?'}`,
                vscode.TreeItemCollapsibleState.None
            ));
        } catch (e) {
            return [new vscode.TreeItem('Crush not available',
                vscode.TreeItemCollapsibleState.None)];
        }
    }
}

function deactivate() {}
exports.deactivate = deactivate;
