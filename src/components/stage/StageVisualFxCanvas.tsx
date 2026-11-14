import React, { useEffect, useRef, useState } from 'react';

export const FX_NAMES = [
  'Mandala Kaleidoscope',
  'Cyberpunk Soundwave Tunnel',
  'Psychedelic Disco Waves',
  'Diya & Lantern Glow',
  'Laser Matrix Concert',
  'Garba Dandiya Sparks',
  'Equalizer Spectrum Bars',
  'Neon Bass Vortex',
  'Desi Holi Color Splash',
  'Baraat Dhol Heatmap',
  'Retro Synthwave Grid',
  'Wedding Sangeet Fireworks',
  'Liquid Marigold Swirl',
  'Cosmic Starlight Warp',
  'Holographic Indian Paisley',
];

interface Props {
  bpm: number;
  energy: number;
  isPlaying: boolean;
  activeFxIndex: number;
  onFxChange?: (index: number) => void;
}

export const StageVisualFxCanvas: React.FC<Props> = ({
  bpm,
  energy,
  isPlaying,
  activeFxIndex,
  onFxChange,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animFrameRef = useRef<number | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let time = 0;
    const particles: { x: number; y: number; vx: number; vy: number; size: number; color: string; alpha: number }[] = [];
    for (let i = 0; i < 70; i++) {
      particles.push({
        x: Math.random() * 800,
        y: Math.random() * 500,
        vx: (Math.random() - 0.5) * 2,
        vy: (Math.random() - 0.5) * 2,
        size: 2 + Math.random() * 5,
        color: ['#f43f5e', '#fbbf24', '#38bdf8', '#a855f7', '#ec4899', '#f97316'][Math.floor(Math.random() * 6)],
        alpha: 0.3 + Math.random() * 0.7,
      });
    }

    const render = () => {
      const w = canvas.width;
      const h = canvas.height;
      if (w === 0 || h === 0) return;

      const speedFactor = isPlaying ? (bpm / 128) : 0.2;
      time += 0.02 * speedFactor;

      const beatPulse = isPlaying ? (Math.sin(time * (bpm / 60) * Math.PI) * 0.5 + 0.5) : 0.2;

      ctx.fillStyle = '#05040a';
      ctx.fillRect(0, 0, w, h);

      const cx = w / 2;
      const cy = h / 2;

      switch (activeFxIndex % 15) {
        case 0: // Mandala Kaleidoscope
          ctx.save();
          ctx.translate(cx, cy);
          for (let petal = 0; petal < 16; petal++) {
            ctx.rotate((Math.PI * 2) / 16);
            ctx.beginPath();
            const r = (Math.sin(time * 2 + petal) * 30 + 80 + beatPulse * 40) * (0.8 + energy * 0.4);
            ctx.ellipse(r, 0, 45 + beatPulse * 25, 18, 0, 0, Math.PI * 2);
            ctx.strokeStyle = petal % 2 === 0 ? `hsla(${(time * 50 + petal * 22) % 360}, 90%, 60%, 0.8)` : '#fbbf24';
            ctx.lineWidth = 3;
            ctx.stroke();

            // Inner mandala ring
            ctx.beginPath();
            ctx.arc(r * 0.5, 0, 8 + beatPulse * 6, 0, Math.PI * 2);
            ctx.fillStyle = `hsla(${(petal * 25 + time * 30) % 360}, 85%, 55%, 0.6)`;
            ctx.fill();
          }
          ctx.restore();
          break;

        case 1: // Cyberpunk Soundwave Tunnel
          ctx.save();
          for (let ring = 1; ring <= 10; ring++) {
            const z = (ring * 40 + time * 60) % 400;
            const radius = (400 - z) * (0.8 + beatPulse * 0.3);
            ctx.beginPath();
            ctx.arc(cx, cy, Math.max(10, radius), 0, Math.PI * 2);
            ctx.strokeStyle = `hsla(${(ring * 35 + time * 80) % 360}, 90%, 60%, ${1 - z / 400})`;
            ctx.lineWidth = 2 + beatPulse * 3;
            ctx.stroke();
          }
          ctx.restore();
          break;

        case 2: // Psychedelic Disco Waves
          for (let layer = 0; layer < 5; layer++) {
            ctx.beginPath();
            ctx.moveTo(0, cy);
            for (let x = 0; x <= w; x += 10) {
              const y = cy + Math.sin(x * 0.015 + time * 3 + layer) * (30 + layer * 15 + beatPulse * 35);
              ctx.lineTo(x, y);
            }
            ctx.strokeStyle = `hsla(${(layer * 60 + time * 70) % 360}, 85%, 60%, 0.75)`;
            ctx.lineWidth = 4;
            ctx.stroke();
          }
          break;

        case 3: // Diya & Lantern Glow
          particles.forEach((p) => {
            p.y -= 1.2 * speedFactor;
            if (p.y < 0) p.y = h;
            ctx.beginPath();
            const glow = p.size * (1 + beatPulse * 0.8);
            ctx.arc(p.x, p.y, glow, 0, Math.PI * 2);
            const grad = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, glow * 2.5);
            grad.addColorStop(0, '#fef08a');
            grad.addColorStop(0.5, '#f59e0b');
            grad.addColorStop(1, 'transparent');
            ctx.fillStyle = grad;
            ctx.fill();
          });
          break;

        case 4: // Laser Matrix Concert
          ctx.save();
          const laserCount = 8;
          for (let l = 0; l < laserCount; l++) {
            const angle = Math.sin(time * 2 + l * 0.8) * 0.9;
            const lx = cx + Math.sin(angle) * w * 0.8;
            const ly = cy - Math.cos(angle) * h * 0.5;
            ctx.beginPath();
            ctx.moveTo(cx + (l - laserCount / 2) * 50, h * 0.9);
            ctx.lineTo(lx, ly);
            ctx.strokeStyle = `hsla(${(l * 45 + time * 100) % 360}, 100%, 65%, ${0.6 + beatPulse * 0.4})`;
            ctx.lineWidth = 3 + beatPulse * 4;
            ctx.shadowBlur = 15;
            ctx.shadowColor = ctx.strokeStyle;
            ctx.stroke();
          }
          ctx.restore();
          break;

        case 5: // Garba Dandiya Sparks
          ctx.save();
          ctx.translate(cx, cy);
          for (let spark = 0; spark < 36; spark++) {
            const angle = (spark * Math.PI) / 18 + time * 2;
            const dist = 70 + (spark % 3) * 35 + beatPulse * 40;
            const sx = Math.cos(angle) * dist;
            const sy = Math.sin(angle) * dist * 0.6;
            ctx.beginPath();
            ctx.arc(sx, sy, 3 + (spark % 4), 0, Math.PI * 2);
            ctx.fillStyle = spark % 2 === 0 ? '#fbbf24' : '#f43f5e';
            ctx.shadowBlur = 10;
            ctx.shadowColor = ctx.fillStyle;
            ctx.fill();
          }
          ctx.restore();
          break;

        case 6: // Equalizer Spectrum Bars
          const bars = 36;
          const barW = (w * 0.8) / bars;
          const startX = w * 0.1;
          for (let b = 0; b < bars; b++) {
            const barH = (Math.sin(time * 4 + b * 0.4) * 0.5 + 0.5) * (h * 0.4) * (0.4 + beatPulse * 0.8);
            ctx.fillStyle = `hsla(${(b * 10 + time * 60) % 360}, 90%, 55%, 0.85)`;
            ctx.fillRect(startX + b * barW, h * 0.75 - barH, barW - 3, barH);
          }
          break;

        case 7: // Neon Bass Vortex
          ctx.save();
          ctx.translate(cx, cy);
          ctx.rotate(time * 1.5);
          for (let v = 0; v < 8; v++) {
            ctx.beginPath();
            for (let a = 0; a < Math.PI * 4; a += 0.1) {
              const r = a * 15 * (1 + beatPulse * 0.3);
              const vx = Math.cos(a + v * (Math.PI / 4)) * r;
              const vy = Math.sin(a + v * (Math.PI / 4)) * r;
              if (a === 0) ctx.moveTo(vx, vy);
              else ctx.lineTo(vx, vy);
            }
            ctx.strokeStyle = `hsla(${(v * 45 + time * 90) % 360}, 90%, 65%, 0.6)`;
            ctx.lineWidth = 2;
            ctx.stroke();
          }
          ctx.restore();
          break;

        case 8: // Desi Holi Color Splash
          particles.forEach((p, idx) => {
            p.x += p.vx * (1 + beatPulse * 2);
            p.y += p.vy * (1 + beatPulse * 2);
            if (p.x < 0 || p.x > w) p.vx *= -1;
            if (p.y < 0 || p.y > h) p.vy *= -1;
            ctx.beginPath();
            ctx.arc(p.x, p.y, p.size * (1 + beatPulse * 0.5), 0, Math.PI * 2);
            ctx.fillStyle = p.color;
            ctx.shadowBlur = 12;
            ctx.shadowColor = p.color;
            ctx.fill();
          });
          break;

        case 9: // Baraat Dhol Heatmap
          for (let r = 1; r <= 6; r++) {
            const rad = ((time * 80 + r * 60) % 300) * (0.8 + beatPulse * 0.4);
            ctx.beginPath();
            ctx.arc(cx, cy, rad, 0, Math.PI * 2);
            ctx.strokeStyle = `rgba(244, 63, 94, ${1 - rad / 300})`;
            ctx.lineWidth = 4 + beatPulse * 6;
            ctx.stroke();
          }
          break;

        case 10: // Retro Synthwave Grid
          ctx.save();
          // Wireframe floor
          for (let x = 0; x <= w; x += 40) {
            ctx.beginPath();
            ctx.moveTo(x, h * 0.55);
            ctx.lineTo((x - cx) * 3 + cx, h);
            ctx.strokeStyle = '#ec4899';
            ctx.lineWidth = 1.5;
            ctx.stroke();
          }
          for (let y = h * 0.55; y <= h; y += 25) {
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(w, y);
            ctx.strokeStyle = '#8b5cf6';
            ctx.lineWidth = 1.5;
            ctx.stroke();
          }
          // Glowing Sun
          const sunRadius = 60 + beatPulse * 15;
          const sunGrad = ctx.createRadialGradient(cx, h * 0.4, 0, cx, h * 0.4, sunRadius);
          sunGrad.addColorStop(0, '#fde047');
          sunGrad.addColorStop(0.7, '#f43f5e');
          sunGrad.addColorStop(1, 'transparent');
          ctx.fillStyle = sunGrad;
          ctx.beginPath();
          ctx.arc(cx, h * 0.4, sunRadius, 0, Math.PI * 2);
          ctx.fill();
          ctx.restore();
          break;

        case 11: // Wedding Sangeet Fireworks
          for (let fw = 0; fw < 4; fw++) {
            const fx = cx + Math.sin(time + fw) * (w * 0.3);
            const fy = cy * 0.7 + Math.cos(time * 1.5 + fw) * 60;
            const fwRadius = 35 + beatPulse * 45;
            for (let ray = 0; ray < 12; ray++) {
              const ang = (ray * Math.PI) / 6 + time;
              ctx.beginPath();
              ctx.moveTo(fx, fy);
              ctx.lineTo(fx + Math.cos(ang) * fwRadius, fy + Math.sin(ang) * fwRadius);
              ctx.strokeStyle = ['#fbbf24', '#f43f5e', '#38bdf8', '#c084fc'][fw % 4];
              ctx.lineWidth = 2;
              ctx.stroke();
            }
          }
          break;

        case 12: // Liquid Marigold Swirl
          ctx.save();
          ctx.translate(cx, cy);
          for (let petal = 0; petal < 24; petal++) {
            const a = (petal * Math.PI) / 12 + time * 1.2;
            const dist = (petal * 12 + time * 30) % 250;
            const px = Math.cos(a) * dist;
            const py = Math.sin(a) * dist * 0.8;
            ctx.beginPath();
            ctx.ellipse(px, py, 14, 8, a, 0, Math.PI * 2);
            ctx.fillStyle = petal % 2 === 0 ? '#f59e0b' : '#fbbf24';
            ctx.shadowBlur = 8;
            ctx.shadowColor = '#d97706';
            ctx.fill();
          }
          ctx.restore();
          break;

        case 13: // Cosmic Starlight Warp
          ctx.save();
          ctx.translate(cx, cy);
          particles.forEach((p, idx) => {
            const ang = (idx * Math.PI * 2) / particles.length;
            const dist = ((time * 150 + idx * 30) % 350) * (0.8 + beatPulse * 0.3);
            const px = Math.cos(ang) * dist;
            const py = Math.sin(ang) * dist;
            ctx.beginPath();
            ctx.arc(px, py, (dist / 350) * 5, 0, Math.PI * 2);
            ctx.fillStyle = `hsl(${(idx * 15 + time * 50) % 360}, 90%, 75%)`;
            ctx.fill();
          });
          ctx.restore();
          break;

        default: // Holographic Indian Paisley
          ctx.save();
          ctx.translate(cx, cy);
          ctx.rotate(time * 0.5);
          for (let p = 0; p < 8; p++) {
            ctx.rotate((Math.PI * 2) / 8);
            ctx.beginPath();
            ctx.moveTo(0, 0);
            ctx.bezierCurveTo(40, -40, 90 + beatPulse * 30, -20, 70, 40);
            ctx.bezierCurveTo(50, 70, 10, 50, 0, 0);
            ctx.strokeStyle = `hsla(${(p * 45 + time * 70) % 360}, 85%, 60%, 0.8)`;
            ctx.lineWidth = 3;
            ctx.stroke();
          }
          ctx.restore();
          break;
      }

      animFrameRef.current = requestAnimationFrame(render);
    };

    render();

    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
      }
    };
  }, [bpm, energy, isPlaying, activeFxIndex]);

  return (
    <div
      onClick={() => onFxChange && onFxChange((activeFxIndex + 1) % 15)}
      className="relative w-full h-full cursor-pointer overflow-hidden"
      title="Click to cycle next Video FX loop (15 Presets)"
    >
      <canvas
        ref={canvasRef}
        width={960}
        height={540}
        className="w-full h-full object-cover"
      />
    </div>
  );
};

