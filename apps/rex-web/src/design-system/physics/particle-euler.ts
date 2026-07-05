import type { Particle2D } from "./types";

export interface ParticlePoolConfig {
  count: number;
  drag?: number;
  gravity?: number;
}

export function createParticlePool(count: number): Particle2D[] {
  return Array.from({ length: count }, () => ({
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    life: 0,
    maxLife: 1,
    active: false,
  }));
}

export function spawnParticle(
  pool: Particle2D[],
  x: number,
  y: number,
  angle: number,
  speed: number,
  life = 1.2,
): void {
  const slot = pool.find((p) => !p.active);
  if (!slot) return;
  slot.x = x;
  slot.y = y;
  slot.vx = Math.cos(angle) * speed;
  slot.vy = Math.sin(angle) * speed;
  slot.life = life;
  slot.maxLife = life;
  slot.active = true;
}

export function spawnBurst(
  pool: Particle2D[],
  x: number,
  y: number,
  count: number,
  spread = Math.PI * 2,
  baseAngle = -Math.PI / 2,
): void {
  for (let i = 0; i < count; i += 1) {
    const angle = baseAngle + (spread * (i / count - 0.5));
    const speed = 40 + Math.random() * 80;
    spawnParticle(pool, x, y, angle, speed, 0.8 + Math.random() * 0.6);
  }
}

export function stepParticles(
  pool: Particle2D[],
  dt: number,
  drag = 2.5,
  gravity = 0,
): void {
  for (const p of pool) {
    if (!p.active) continue;
    p.life -= dt;
    if (p.life <= 0) {
      p.active = false;
      continue;
    }
    p.vx *= Math.exp(-drag * dt);
    p.vy = p.vy * Math.exp(-drag * dt) + gravity * dt;
    p.x += p.vx * dt;
    p.y += p.vy * dt;
  }
}

export function activeParticleCount(pool: Particle2D[]): number {
  return pool.filter((p) => p.active).length;
}
