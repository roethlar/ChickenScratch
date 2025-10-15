import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as yaml from 'js-yaml';

/**
 * ChickenScratch VS Code Extension
 *
 * Provides native .chikn project support in VS Code/code-server.
 */

export function activate(context: vscode.ExtensionContext) {
    console.log('ChickenScratch extension activated');

    // Register tree view provider
    const treeProvider = new ChiknTreeProvider();
    vscode.window.registerTreeDataProvider('chickenscratch.projectTree', treeProvider);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('chickenscratch.newProject', () => createNewProject()),
        vscode.commands.registerCommand('chickenscratch.newDocument', (node?: ChiknTreeItem) => createNewDocument(node, treeProvider)),
        vscode.commands.registerCommand('chickenscratch.newFolder', (node?: ChiknTreeItem) => createNewFolder(node, treeProvider)),
        vscode.commands.registerCommand('chickenscratch.deleteDocument', (node: ChiknTreeItem) => deleteDocument(node, treeProvider)),
        vscode.commands.registerCommand('chickenscratch.refreshTree', () => treeProvider.refresh()),
        vscode.commands.registerCommand('chickenscratch.openDocument', (node: ChiknTreeItem) => openDocument(node))
    );

    // Auto-detect .chikn projects in workspace
    if (vscode.workspace.workspaceFolders) {
        for (const folder of vscode.workspace.workspaceFolders) {
            const chiknProjects = findChiknProjects(folder.uri.fsPath);
            if (chiknProjects.length > 0) {
                treeProvider.refresh();
            }
        }
    }
}

export function deactivate() {}

/**
 * Tree data provider for .chikn projects
 */
class ChiknTreeProvider implements vscode.TreeDataProvider<ChiknTreeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<ChiknTreeItem | undefined> = new vscode.EventEmitter<ChiknTreeItem | undefined>();
    readonly onDidChangeTreeData: vscode.Event<ChiknTreeItem | undefined> = this._onDidChangeTreeData.event;

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: ChiknTreeItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: ChiknTreeItem): Thenable<ChiknTreeItem[]> {
        if (!vscode.workspace.workspaceFolders) {
            return Promise.resolve([]);
        }

        if (!element) {
            // Root level - find all .chikn projects
            const chiknProjects: ChiknTreeItem[] = [];
            for (const folder of vscode.workspace.workspaceFolders) {
                const projects = findChiknProjects(folder.uri.fsPath);
                chiknProjects.push(...projects.map(p => new ChiknTreeItem(
                    path.basename(p),
                    vscode.TreeItemCollapsibleState.Expanded,
                    p,
                    'project'
                )));
            }
            return Promise.resolve(chiknProjects);
        }

        // Load project hierarchy from project.yaml
        return Promise.resolve(this.getProjectChildren(element));
    }

    private getProjectChildren(element: ChiknTreeItem): ChiknTreeItem[] {
        const projectPath = element.type === 'project' ? element.resourcePath : this.findProjectRoot(element.resourcePath);
        if (!projectPath) {
            return [];
        }

        const projectYaml = path.join(projectPath, 'project.yaml');
        if (!fs.existsSync(projectYaml)) {
            return [];
        }

        try {
            const content = fs.readFileSync(projectYaml, 'utf8');
            const project = yaml.load(content) as ChiknProject;

            if (element.type === 'project') {
                // Return root hierarchy nodes
                return project.hierarchy.map(node => this.treeNodeToTreeItem(node, projectPath));
            } else if (element.type === 'folder') {
                // Find this folder in hierarchy and return its children
                const node = this.findNodeInHierarchy(project.hierarchy, element.id!);
                if (node && node.type === 'Folder' && node.children) {
                    return node.children.map(child => this.treeNodeToTreeItem(child, projectPath));
                }
            }
        } catch (err) {
            console.error('Failed to parse project.yaml:', err);
        }

        return [];
    }

    private treeNodeToTreeItem(node: TreeNode, projectPath: string): ChiknTreeItem {
        if (node.type === 'Folder') {
            return new ChiknTreeItem(
                node.name,
                vscode.TreeItemCollapsibleState.Collapsed,
                projectPath,
                'folder',
                node.id
            );
        } else {
            // Document node
            const docPath = path.join(projectPath, node.path || '');
            return new ChiknTreeItem(
                node.name,
                vscode.TreeItemCollapsibleState.None,
                docPath,
                'document',
                node.id,
                {
                    command: 'chickenscratch.openDocument',
                    title: 'Open Document',
                    arguments: [node]
                }
            );
        }
    }

    private findNodeInHierarchy(hierarchy: TreeNode[], id: string): TreeNode | null {
        for (const node of hierarchy) {
            if (node.id === id) {
                return node;
            }
            if (node.type === 'Folder' && node.children) {
                const found = this.findNodeInHierarchy(node.children, id);
                if (found) {
                    return found;
                }
            }
        }
        return null;
    }

    private findProjectRoot(resourcePath: string): string | null {
        let current = resourcePath;
        while (current !== path.dirname(current)) {
            if (current.endsWith('.chikn')) {
                return current;
            }
            current = path.dirname(current);
        }
        return null;
    }
}

/**
 * Tree item for ChickenScratch documents/folders
 */
class ChiknTreeItem extends vscode.TreeItem {
    public readonly type: 'project' | 'folder' | 'document';
    public readonly id?: string;
    public readonly resourcePath: string;

    constructor(
        label: string,
        collapsibleState: vscode.TreeItemCollapsibleState,
        resourcePath: string,
        type: 'project' | 'folder' | 'document',
        id?: string,
        command?: vscode.Command
    ) {
        super(label, collapsibleState);
        this.type = type;
        this.id = id;
        this.resourcePath = resourcePath;
        this.contextValue = type;
        this.command = command;

        // Set resourceUri as Uri type
        if (type === 'document') {
            this.resourceUri = vscode.Uri.file(resourcePath);
            this.iconPath = new vscode.ThemeIcon('file-text');
        } else if (type === 'folder') {
            this.iconPath = new vscode.ThemeIcon('folder');
        } else {
            this.iconPath = new vscode.ThemeIcon('book');
        }
    }
}

/**
 * Find all .chikn projects in a directory
 */
function findChiknProjects(rootPath: string): string[] {
    const projects: string[] = [];

    try {
        const entries = fs.readdirSync(rootPath, { withFileTypes: true });
        for (const entry of entries) {
            if (entry.isDirectory() && entry.name.endsWith('.chikn')) {
                projects.push(path.join(rootPath, entry.name));
            }
        }
    } catch (err) {
        console.error('Error scanning for .chikn projects:', err);
    }

    return projects;
}

/**
 * Create a new .chikn project
 */
async function createNewProject() {
    const name = await vscode.window.showInputBox({
        prompt: 'Project name',
        placeHolder: 'My Novel'
    });

    if (!name) {
        return;
    }

    const folderUri = await vscode.window.showOpenDialog({
        canSelectFiles: false,
        canSelectFolders: true,
        canSelectMany: false,
        openLabel: 'Create Project Here'
    });

    if (!folderUri || folderUri.length === 0) {
        return;
    }

    const projectPath = path.join(folderUri[0].fsPath, `${name}.chikn`);

    // Create project structure
    fs.mkdirSync(projectPath, { recursive: true });
    fs.mkdirSync(path.join(projectPath, 'manuscript'), { recursive: true });
    fs.mkdirSync(path.join(projectPath, 'research'), { recursive: true });
    fs.mkdirSync(path.join(projectPath, 'templates'), { recursive: true });
    fs.mkdirSync(path.join(projectPath, 'settings'), { recursive: true });

    // Create project.yaml
    const project: ChiknProject = {
        id: generateId(),
        name: name,
        hierarchy: [],
        created: new Date().toISOString(),
        modified: new Date().toISOString()
    };

    const projectYaml = yaml.dump(project);
    fs.writeFileSync(path.join(projectPath, 'project.yaml'), projectYaml, 'utf8');

    // Initialize git repository
    const gitPath = path.join(projectPath, '.git');
    if (!fs.existsSync(gitPath)) {
        try {
            await vscode.commands.executeCommand('git.init', vscode.Uri.file(projectPath));

            // Create .gitignore
            const gitignore = `.DS_Store\nThumbs.db\n`;
            fs.writeFileSync(path.join(projectPath, '.gitignore'), gitignore, 'utf8');

            // Initial commit
            const gitExtension = vscode.extensions.getExtension('vscode.git')?.exports;
            if (gitExtension) {
                const api = gitExtension.getAPI(1);
                const repo = api.repositories.find((r: any) => r.rootUri.fsPath === projectPath);
                if (repo) {
                    await repo.add([]);  // Stage all
                    await repo.commit('Initial commit: Project created');
                }
            }
        } catch (err) {
            console.error('Failed to initialize git:', err);
            vscode.window.showWarningMessage('Project created but git initialization failed');
        }
    }

    // Open project in workspace
    vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(projectPath), false);

    vscode.window.showInformationMessage(`Created project: ${name}`);
}

/**
 * Create a new document
 */
async function createNewDocument(parentNode: ChiknTreeItem | undefined, treeProvider: ChiknTreeProvider) {
    const name = await vscode.window.showInputBox({
        prompt: 'Document name',
        placeHolder: 'Chapter 1'
    });

    if (!name) {
        return;
    }

    // Find project root
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }

    const projectPath = workspaceFolder.uri.fsPath;
    const projectYamlPath = path.join(projectPath, 'project.yaml');

    if (!fs.existsSync(projectYamlPath)) {
        vscode.window.showErrorMessage('Not a .chikn project');
        return;
    }

    // Read project
    const content = fs.readFileSync(projectYamlPath, 'utf8');
    const project = yaml.load(content) as ChiknProject;

    // Create slug for filename
    const slug = slugify(name);
    const docId = generateId();
    const docPath = `manuscript/${slug}.md`;
    const fullDocPath = path.join(projectPath, docPath);
    const metaPath = path.join(projectPath, `manuscript/${slug}.meta`);

    // Create document files
    fs.writeFileSync(fullDocPath, `# ${name}\n\n`, 'utf8');

    const metadata = {
        id: docId,
        name: name,
        created: new Date().toISOString(),
        modified: new Date().toISOString(),
        parent_id: parentNode?.id || null
    };

    fs.writeFileSync(metaPath, yaml.dump(metadata), 'utf8');

    // Add to project hierarchy
    const newNode: TreeNode = {
        type: 'Document',
        id: docId,
        name: name,
        path: docPath
    };

    if (parentNode && parentNode.type === 'folder') {
        // Add to folder's children (would need to traverse and update)
        project.hierarchy.push(newNode); // Simplified: add to root
    } else {
        project.hierarchy.push(newNode);
    }

    project.modified = new Date().toISOString();

    // Save project
    fs.writeFileSync(projectYamlPath, yaml.dump(project), 'utf8');

    // Refresh tree
    treeProvider.refresh();

    // Open document
    vscode.workspace.openTextDocument(fullDocPath).then(doc => {
        vscode.window.showTextDocument(doc);
    });
}

/**
 * Create a new folder
 */
async function createNewFolder(parentNode: ChiknTreeItem | undefined, treeProvider: ChiknTreeProvider) {
    const name = await vscode.window.showInputBox({
        prompt: 'Folder name',
        placeHolder: 'Part One'
    });

    if (!name) {
        return;
    }

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }

    const projectPath = workspaceFolder.uri.fsPath;
    const projectYamlPath = path.join(projectPath, 'project.yaml');

    const content = fs.readFileSync(projectYamlPath, 'utf8');
    const project = yaml.load(content) as ChiknProject;

    const newFolder: TreeNode = {
        type: 'Folder',
        id: generateId(),
        name: name,
        children: []
    };

    project.hierarchy.push(newFolder);
    project.modified = new Date().toISOString();

    fs.writeFileSync(projectYamlPath, yaml.dump(project), 'utf8');
    treeProvider.refresh();

    vscode.window.showInformationMessage(`Created folder: ${name}`);
}

/**
 * Delete a document or folder
 */
async function deleteDocument(node: ChiknTreeItem, treeProvider: ChiknTreeProvider) {
    const confirm = await vscode.window.showWarningMessage(
        `Delete "${node.label}"?`,
        { modal: true },
        'Delete'
    );

    if (confirm !== 'Delete') {
        return;
    }

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return;
    }

    const projectPath = workspaceFolder.uri.fsPath;
    const projectYamlPath = path.join(projectPath, 'project.yaml');

    const content = fs.readFileSync(projectYamlPath, 'utf8');
    const project = yaml.load(content) as ChiknProject;

    // Remove from hierarchy (simplified - only handles root level)
    project.hierarchy = project.hierarchy.filter(n => n.id !== node.id);
    project.modified = new Date().toISOString();

    // Delete files if document
    if (node.type === 'document') {
        if (fs.existsSync(node.resourcePath)) {
            fs.unlinkSync(node.resourcePath);
        }
        // Delete .meta file
        const metaPath = node.resourcePath.replace('.md', '.meta');
        if (fs.existsSync(metaPath)) {
            fs.unlinkSync(metaPath);
        }
    }

    fs.writeFileSync(projectYamlPath, yaml.dump(project), 'utf8');
    treeProvider.refresh();
}

/**
 * Open a document in the editor
 */
function openDocument(node: ChiknTreeItem) {
    if (node.type === 'document') {
        vscode.workspace.openTextDocument(node.resourcePath).then(doc => {
            vscode.window.showTextDocument(doc);
        });
    }
}

/**
 * Generate a unique ID
 */
function generateId(): string {
    return Math.random().toString(36).substring(2, 15);
}

/**
 * Convert string to URL-safe slug
 */
function slugify(text: string): string {
    return text
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/(^-|-$)/g, '');
}

/**
 * Type definitions for .chikn format
 */

interface ChiknProject {
    id: string;
    name: string;
    hierarchy: TreeNode[];
    created: string;
    modified: string;
}

interface TreeNode {
    type: 'Document' | 'Folder';
    id: string;
    name: string;
    path?: string;
    children?: TreeNode[];
}
