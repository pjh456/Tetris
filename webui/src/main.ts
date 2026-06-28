import './style.css';
import { effect, untracked } from '@preact/signals-core';
import { page, settings, is_multiplayer, connection_status } from './state';
import { apply_theme, type ThemeName } from './core/theme';
import { init_particles, apply_theme_particles } from './core/particles';
import { init_wasm } from './core/wasm';
import { create_home_screen } from './screens/home';
import { create_game_screen } from './screens/game';
import { create_settings_screen } from './screens/settings';
import { create_gameover_screen } from './screens/gameover';
import { create_leaderboard_screen } from './screens/leaderboard';
import { create_lobby_screen } from './screens/lobby';
import { create_spectator_screen } from './screens/spectator';
import { create_multiplayer_result_screen } from './screens/multiplayer_result';
import { create_watch_ai_screen } from './screens/watch_ai';
import { reset_multiplayer_ws } from './core/multiplayer';
import { get_display_name } from './core/user_profile';
import { create_connect_dialog } from './screens/connect_dialog';
import { is_offline } from './core/server_config';

const CONNECTION_LABELS: Record<string, string> = {
  offline: 'OFFLINE',
  connecting: 'CONNECTING',
  online: 'ONLINE',
  slow: 'SLOW',
  reconnecting: 'RECONNECTING',
  disconnected: 'DISCONNECTED',
  resyncing: 'RESYNCING',
};

const boot_theme = (settings.value.theme as ThemeName) || 'cyberpunk';
apply_theme(boot_theme);
init_particles(boot_theme);

effect(() => {
  const theme = (settings.value.theme as ThemeName) || 'cyberpunk';
  apply_theme(theme);
  apply_theme_particles(theme);
});

const app = document.getElementById('app')!;

let _zoom = 1.0;
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2.0;
const ZOOM_STEP = 0.1;

function apply_zoom(delta: number) {
  _zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, _zoom + delta * ZOOM_STEP));
  app.style.zoom = String(_zoom);
}

document.addEventListener('wheel', (e) => {
  if (!e.ctrlKey) return;
  e.preventDefault();
  apply_zoom(-Math.sign(e.deltaY));
}, { passive: false });

let _pinch_dist: number | null = null;
document.addEventListener('touchstart', (e) => {
  if (e.touches.length === 2) {
    const dx = e.touches[0].clientX - e.touches[1].clientX;
    const dy = e.touches[0].clientY - e.touches[1].clientY;
    _pinch_dist = Math.hypot(dx, dy);
  }
});
document.addEventListener('touchmove', (e) => {
  if (_pinch_dist === null || e.touches.length !== 2) return;
  const dx = e.touches[0].clientX - e.touches[1].clientX;
  const dy = e.touches[0].clientY - e.touches[1].clientY;
  const dist = Math.hypot(dx, dy);
  const ratio = dist / _pinch_dist;
  if (ratio > 1.1) { apply_zoom(1); _pinch_dist = dist; }
  else if (ratio < 0.9) { apply_zoom(-1); _pinch_dist = dist; }
});
document.addEventListener('touchend', () => { _pinch_dist = null; });

function mount_topbar(container: HTMLElement) {
  const topbar = document.createElement('div');
  topbar.className = 'topbar';
  topbar.innerHTML = `
    <div class="topbar-left">
      <span class="back-btn" title="Back to Home">←</span>
      <div class="logo">TETRIS</div>
    </div>
    <div class="user-info">
      <div class="user-name"></div>
      <div class="status">OFFLINE</div>
    </div>
  `;
  // Display name from local profile (textContent — avoids HTML injection).
  topbar.querySelector('.user-name')!.textContent = get_display_name();
  // Status reflects the live connection_status signal.
  const status_el = topbar.querySelector('.status')!;
  const dispose = effect(() => {
    const status = connection_status.value;
    status_el.textContent = CONNECTION_LABELS[status] ?? status.toUpperCase();
  });
  _cleanups.push(dispose);
  topbar.querySelector('.back-btn')!.addEventListener('click', () => {
    page.value = 'home';
  });
  container.prepend(topbar);
}

function mount_back_button(container: HTMLElement) {
  const btn = document.createElement('button');
  btn.className = 'back-float';
  btn.textContent = '← Back';
  btn.title = 'Back to Home';
  btn.addEventListener('click', () => {
    page.value = 'home';
  });
  container.appendChild(btn);
}

type CleanableElement = HTMLElement & { _cleanup?: () => void };
let active_lobby: CleanableElement | null = null;
let active_game: CleanableElement | null = null;
let _cleanups: (() => void)[] = [];

function open_multiplayer() {
  if (connection_status.value === 'online') {
    page.value = 'lobby';
    return;
  }
  const dispose = create_connect_dialog({
    on_connected: () => {
      page.value = 'lobby';
    },
    on_offline: () => {},
  });
  _cleanups.push(dispose);
}

effect(() => {
  const current = page.value;
  active_lobby?._cleanup?.();
  active_lobby = null;
  active_game?._cleanup?.();
  active_game = null;
  for (const fn of _cleanups) fn();
  _cleanups = [];
  app.innerHTML = '';

  // Reset multiplayer state when leaving multiplayer screens
  if (current === 'home' || current === 'game') {
    is_multiplayer.value = false;
  }

  // Landing on home is the single exit point of any multiplayer session →
  // tear down the ws + heartbeat there (centralized). Not on 'game': a
  // lobby→game handoff keeps the ws alive (closing it would break the game).
  if (current === 'home') {
    reset_multiplayer_ws();
  }

  switch (current) {
    case 'home': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      content.appendChild(
        create_home_screen(
          () => {
            page.value = 'game';
          },
          open_multiplayer,
          () => {
            page.value = 'settings';
          },
        ),
      );
      app.appendChild(content);
      // First visit / not connected: prompt for a relay server unless the user
      // explicitly chose offline mode. Modal forces connect / LAN / offline.
      if (!is_offline() && connection_status.value !== 'online') {
        const dispose = create_connect_dialog({
          on_connected: () => {},
          on_offline: () => {},
        });
        _cleanups.push(dispose);
      }
      break;
    }
    case 'game': {
      const content = document.createElement('div') as CleanableElement;
      content.className = 'content';
      app.appendChild(content);
      let cancelled = false;
      create_game_screen(content)
        .then((destroy) => {
          content._cleanup = destroy;
          if (cancelled) {
            destroy();
          }
        })
        .catch(() => {});
      content._cleanup = () => {
        cancelled = true;
      };
      active_game = content;
      mount_back_button(app);
      break;
    }
    case 'settings': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      const settings_el = create_settings_screen() as CleanableElement;
      content.appendChild(settings_el);
      app.appendChild(content);
      _cleanups.push(() => settings_el._cleanup?.());
      break;
    }
    case 'gameover': {
      const go_el = create_gameover_screen() as CleanableElement;
      app.appendChild(go_el);
      mount_back_button(app);
      _cleanups.push(() => go_el._cleanup?.());
      break;
    }
    case 'leaderboard': {
      app.appendChild(create_leaderboard_screen());
      break;
    }
    case 'lobby': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      app.appendChild(content);

      void (async () => {
        try {
          await init_wasm(content);
          if (page.value !== 'lobby') {
            return;
          }
          const lobby_el = untracked(() => create_lobby_screen()) as CleanableElement;
          active_lobby = lobby_el;
          content.innerHTML = '';
          content.appendChild(lobby_el);
        } catch {
          // `init_wasm` already renders error state into `content`.
        }
      })();
      break;
    }
    case 'spectator': {
      const sp_el = create_spectator_screen() as CleanableElement;
      app.appendChild(sp_el);
      mount_back_button(app);
      _cleanups.push(() => sp_el._cleanup?.());
      break;
    }
    case 'multiplayer_result': {
      mount_topbar(app);
      const content = document.createElement('div');
      content.className = 'content';
      content.appendChild(create_multiplayer_result_screen());
      app.appendChild(content);
      break;
    }
    case 'watch_ai': {
      const content = document.createElement('div');
      content.className = 'content';
      app.appendChild(content);
      void (async () => {
        try {
          await init_wasm(content);
          if (page.value !== 'watch_ai') {
            return;
          }
          const destroy = untracked(() => create_watch_ai_screen(content));
          _cleanups.push(destroy);
        } catch {
          // `init_wasm` already renders error state into `content`.
        }
      })();
      mount_back_button(app);
      break;
    }
  }
});
