export async function init_particles(): Promise<void> {
  const container = document.createElement('div');
  container.id = 'particles-bg';
  container.style.position = 'fixed';
  container.style.inset = '0';
  container.style.zIndex = '-1';
  container.style.pointerEvents = 'none';
  document.body.prepend(container);
}

export function destroy_particles(): void {
  const el = document.getElementById('particles-bg');
  if (el) el.remove();
}
