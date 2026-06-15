<script lang="ts">
  import { page } from '$app/stores';
  import Icon from './Icon.svelte';

  // Deployments-first IA: local runs are the daily driver; Containers is demoted
  // to the bottom (rarely populated on Windows since runs are native).
  const links = [
    { href: '/dashboard', icon: 'dashboard', label: 'Dashboard' },
    { href: '/deployments', icon: 'rocket', label: 'Deployments' },
    { href: '/services', icon: 'services', label: 'Services' },
    { href: '/images', icon: 'images', label: 'Images' },
    { href: '/logs', icon: 'logs', label: 'Logs' },
    { href: '/mailbox', icon: 'mail', label: 'Mailbox' },
  ];

  const bottomLinks = [
    { href: '/containers', icon: 'containers', label: 'Containers' },
    { href: '/docs', icon: 'docs', label: 'Docs' },
    { href: '/settings', icon: 'settings', label: 'Settings' },
  ];
</script>

<nav class="sidebar">
  <div class="nav-links">
    {#each links as link}
      <a
        href={link.href}
        class="nav-item"
        class:active={$page.url.pathname === link.href}
        title={link.label}
      >
        <span class="nav-icon"><Icon name={link.icon} size={18} /></span>
        <span class="nav-tooltip">{link.label}</span>
      </a>
    {/each}
  </div>

  <div class="spacer"></div>

  <div class="divider"></div>

  <div class="nav-links bottom">
    {#each bottomLinks as link}
      <a
        href={link.href}
        class="nav-item"
        class:active={$page.url.pathname === link.href}
        title={link.label}
      >
        <span class="nav-icon"><Icon name={link.icon} size={18} /></span>
        <span class="nav-tooltip">{link.label}</span>
      </a>
    {/each}
  </div>
</nav>

<style>
  .sidebar {
    width: 48px;
    height: 100vh;
    background: var(--color-crush-dark);
    border-right: 1px solid var(--color-crush-border);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 0;
    position: fixed;
    left: 0;
    top: 0;
    z-index: 50;
  }

  .nav-links {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    align-items: center;
  }

  .nav-item {
    position: relative;
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    color: var(--color-crush-text-muted);
    text-decoration: none;
    transition: all 0.15s;
  }

  .nav-item:hover {
    color: white;
    background: rgba(26, 26, 34, 0.5);
  }

  .nav-item.active {
    background: rgba(255, 255, 255, 0.08);
    border-left: 2px solid var(--color-crush-text);
    color: var(--color-crush-text);
    border-radius: 0 8px 8px 0;
    width: 34px;
  }

  .nav-icon {
    font-size: 16px;
  }

  .nav-tooltip {
    display: none;
    position: absolute;
    left: calc(100% + 8px);
    background: var(--color-crush-surface);
    color: var(--color-crush-text);
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    white-space: nowrap;
    border: 1px solid var(--color-crush-border);
    pointer-events: none;
    z-index: 100;
  }

  .nav-item:hover .nav-tooltip {
    display: block;
  }

  .spacer {
    flex: 1;
  }

  .bottom {
    margin-top: auto;
  }
</style>
