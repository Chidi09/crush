<script lang="ts">
  import { page } from '$app/stores';
  import Icon from './Icon.svelte';

  const links = [
    { href: '/dashboard', icon: 'dashboard', label: 'Dashboard' },
    { href: '/containers', icon: 'containers', label: 'Containers' },
    { href: '/services', icon: 'services', label: 'Services' },
    { href: '/images', icon: 'images', label: 'Images' },
    { href: '/logs', icon: 'logs', label: 'Logs' },
    { href: '/build', icon: 'build', label: 'Build' },
  ];

  const bottomLinks = [
    { href: '/settings', icon: 'settings', label: 'Settings' },
  ];
</script>

<nav class="sidebar">
  <div class="logo">
    <svg class="logo-mark" width="22" height="22" viewBox="0 0 32 32" aria-hidden="true">
      <path d="M16 3 L28 16 L16 29 L4 16 Z" fill="var(--color-crush-orange)" />
    </svg>
  </div>

  <div class="divider"></div>

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
    padding: 8px 0;
    position: fixed;
    left: 0;
    top: 0;
    z-index: 50;
  }

  .logo {
    padding: 8px 0;
  }

  .logo-mark {
    font-size: 20px;
    color: var(--color-crush-orange);
  }

  .divider {
    width: 24px;
    height: 1px;
    background: var(--color-crush-border);
    margin: 8px 0;
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
    background: rgba(224, 85, 64, 0.1);
    border-left: 2px solid var(--color-crush-orange);
    color: var(--color-crush-orange);
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
