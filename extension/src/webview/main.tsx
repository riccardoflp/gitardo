import '@vscode/codicons/dist/codicon.css';
import './styles.css';
import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import type { ViewKind } from '../shared/protocol';
import { MenuProvider } from './ContextMenu';

// What the extension wrote on the page for the view it is.
const { view: kind, commit = '', root = '', files } = document.body.dataset;

// Each view imports only its own page: the Changes view never loads the graph, nor the graph the review of conflicts.
const VIEWS: Record<ViewKind, () => Promise<ReactNode>> = {
  graph: async () => {
    const { App } = await import('./App');
    return <App />;
  },
  commits: async () => {
    const { CommitsApp } = await import('./CommitsApp');
    return <CommitsApp />;
  },
  // The graph as it fits beside the editor: without the references, which have a view of their own there.
  sideGraph: async () => {
    const { App } = await import('./App');
    return <App compact />;
  },
  changes: async () => {
    const { ChangesApp } = await import('./ChangesApp');
    return <ChangesApp />;
  },
  branches: async () => {
    const { BranchesApp } = await import('./BranchesApp');
    return <BranchesApp />;
  },
  stashes: async () => {
    const { StashesApp } = await import('./StashesApp');
    return <StashesApp />;
  },
  commit: async () => {
    const { CommitApp } = await import('./CommitApp');
    return <CommitApp hash={commit} root={root} layout={files === 'list' ? 'list' : 'tree'} />;
  },
  compare: async () => {
    const { CompareApp } = await import('./CompareApp');
    return <CompareApp root={root} layout={files === 'list' ? 'list' : 'tree'} />;
  },
  worktreeSetup: async () => {
    const { WorktreeSetupApp } = await import('./WorktreeSetupApp');
    return <WorktreeSetupApp />;
  },
  conflicts: async () => {
    const { ConflictsApp } = await import('./ConflictsApp');
    return <ConflictsApp />;
  },
  repositories: async () => {
    const { RepositoriesApp } = await import('./RepositoriesApp');
    return <RepositoriesApp />;
  },
  rangeDiff: async () => {
    const { RangeDiffApp } = await import('./RangeDiffApp');
    return <RangeDiffApp />;
  },
};
const view = await (VIEWS[kind as ViewKind] ?? VIEWS.graph)();

createRoot(document.getElementById('root')!).render(<MenuProvider>{view}</MenuProvider>);
