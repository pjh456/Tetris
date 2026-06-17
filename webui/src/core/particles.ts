import { tsParticles, type Container } from '@tsparticles/engine';
import { loadSlim } from '@tsparticles/slim';
import type { ThemeName } from './theme';

let container: Container | undefined = undefined;
let initialized = false;
let init_promise: Promise<void> | null = null;

export async function init_particles(theme: ThemeName = 'cyberpunk'): Promise<void> {
  if (initialized) return;
  if (init_promise) return init_promise;

  init_promise = (async () => {
    await loadSlim(tsParticles);
    initialized = true;

    let el = document.getElementById('particles-bg');
    if (!el) {
      el = document.createElement('div');
      el.id = 'particles-bg';
      el.style.position = 'fixed';
      el.style.inset = '0';
      el.style.zIndex = '-1';
      el.style.pointerEvents = 'none';
      document.body.prepend(el);
    }

    container = await tsParticles.load({
      id: 'particles-bg',
      options: get_config(theme),
    });
  })();

  return init_promise;
}

export async function apply_theme_particles(theme: ThemeName): Promise<void> {
  if (!initialized) return;
  if (container) {
    container.destroy();
    container = undefined;
  }
  container = await tsParticles.load({
    id: 'particles-bg',
    options: get_config(theme),
  });
}

export function destroy_particles(): void {
  container?.destroy();
  container = undefined;
  document.getElementById('particles-bg')?.remove();
  initialized = false;
}

function get_config(theme: ThemeName): Record<string, unknown> {
  const base = {
    fullScreen: { enable: false },
    background: { color: 'transparent' },
    fpsLimit: 30,
    detectRetina: true,
  };

  switch (theme) {
    case 'cyberpunk':
      return {
        ...base,
        particles: {
          number: { value: 40, density: { enable: true } },
          color: { value: '#5de2ff' },
          shape: { type: 'circle' },
          opacity: { value: { min: 0.1, max: 0.3 } },
          size: { value: { min: 1, max: 3 } },
          move: { enable: true, speed: 0.5, direction: 'none' as const, random: true },
          links: { enable: true, distance: 150, color: '#5de2ff', opacity: 0.1 },
        },
      };
    case 'retro':
      return {
        ...base,
        particles: {
          number: { value: 30, density: { enable: true } },
          color: { value: '#e94560' },
          shape: { type: 'square' },
          opacity: { value: { min: 0.2, max: 0.4 } },
          size: { value: { min: 2, max: 5 } },
          move: { enable: true, speed: 0.3, direction: 'none' as const, random: true },
          links: { enable: true, distance: 120, color: '#533483', opacity: 0.15 },
        },
      };
    case 'minimal':
      return {
        ...base,
        particles: {
          number: { value: 20, density: { enable: true } },
          color: { value: '#999999' },
          shape: { type: 'circle' },
          opacity: { value: { min: 0.05, max: 0.2 } },
          size: { value: { min: 1, max: 2 } },
          move: { enable: true, speed: 0.2, direction: 'none' as const, random: true },
          links: { enable: false },
        },
      };
    default:
      return get_config('cyberpunk');
  }
}
